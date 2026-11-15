//! 显式环境选择与 Rustup 调用；不修改进程全局环境或全局默认工具链。
use crate::profile::path as host_path;
use crate::{
    bindings::{self, Declarations, Environment, NativePlatform, Tools},
    verified_bindings::{self as policy, Executable},
};
use chrono_harness::{json, resolve_program};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitStatus,
};

#[derive(Debug)]
pub enum Failure {
    Selection(String),
    Installer(ExitStatus),
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::Selection(message)
    }
}
impl From<&str> for Failure {
    fn from(message: &str) -> Self {
        Self::Selection(message.into())
    }
}
impl std::fmt::Display for Failure {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Selection(message) => out.write_str(message),
            Self::Installer(status) => write!(out, "Rustup failed: {status}"),
        }
    }
}

pub struct Prepared {
    pub environment: Environment,
    pub host_directories: Vec<PathBuf>,
    toolchain: String,
    incremental: Option<bool>,
}

#[derive(Debug)]
pub struct Selected {
    pub environment: Environment,
    pub versions: Tools<String>,
    pub bindings: Option<Tools<Executable>>,
}

fn typed<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

impl Prepared {
    /// 只解析与观察声明的构建环境；不创建目录或启动进程。
    pub fn parse(root: &Path, bytes: &[u8], inherited: &Environment) -> Result<Self, String> {
        let config = json(bytes)?;
        let toolchain: String = typed(&config["rust_toolchain"])?;
        let incremental = config
            .get("rust_incremental")
            .map(typed::<bool>)
            .transpose()?;
        let mut environment = inherited.clone();
        let explicit = config
            .get("environment")
            .map(typed::<BTreeMap<String, String>>)
            .transpose()?
            .unwrap_or_default();
        for (key, value) in &explicit {
            if !policy::environment_key(key.as_bytes())
                || !policy::environment_value(value.as_bytes())
            {
                return Err("invalid bootstrap environment selection".into());
            }
            environment.insert(key.into(), value.into());
        }
        let mut host_directories = Vec::new();
        if let Some(values) = config.get("host_environment") {
            let values: BTreeMap<String, String> = typed(values)?;
            if values.is_empty() {
                return Err("empty bootstrap host environment selection".into());
            }
            for (key, value) in values {
                if !policy::host_variable(&key) || explicit.contains_key(&key) {
                    return Err(
                        "invalid or conflicting bootstrap host environment selection".into(),
                    );
                }
                let selected = host_path(root, &value, b".chrono-harness/cache/")?;
                environment.insert(key.into(), selected.as_os_str().to_owned());
                host_directories.push(selected);
            }
        }
        Ok(Self {
            environment,
            host_directories,
            toolchain,
            incremental,
        })
    }

    /// 调用方先完成显式 prerequisites，再选择工具链及补齐缺少的组件。
    /// 不创建 host_directories；目录建立仍由源码构建调用方负责。
    pub fn activate(
        self,
        root: &Path,
        declarations: Option<&Declarations>,
        configured: &BTreeMap<String, String>,
    ) -> Result<Selected, Failure> {
        let mut environment = self.environment;
        let path = environment
            .get(OsStr::new("PATH"))
            .map(|v| v.to_str().ok_or("non UTF-8 PATH"))
            .transpose()?;
        let rustup = resolve_program(root, "rustup", path)?;
        let install = |args: &[&str]| -> Result<(), Failure> {
            let status = bindings::command(root, &rustup, args, &environment)?
                .status()
                .map_err(|error| error.to_string())?;
            if !status.success() {
                return Err(Failure::Installer(status));
            }
            Ok(())
        };
        let probe = |name: &str| {
            bindings::capture(
                root,
                &rustup,
                &["run", &self.toolchain, name, "--version"],
                &environment,
            )
        };
        for (name, cargo) in [("cargo", true), ("rustfmt", false)] {
            match policy::probe_action(probe(name)?.status.success(), cargo) {
                policy::ProbeAction::Ready => {}
                policy::ProbeAction::InstallMinimal => install(&[
                    "toolchain",
                    "install",
                    &self.toolchain,
                    "--profile",
                    "minimal",
                    "--component",
                    "rustfmt",
                ])?,
                policy::ProbeAction::AddFormat => install(&[
                    "component",
                    "add",
                    "--toolchain",
                    &self.toolchain,
                    "rustfmt",
                ])?,
            }
        }
        environment.insert("RUSTUP_TOOLCHAIN".into(), self.toolchain.clone().into());
        environment.insert("CARGO_TERM_COLOR".into(), "never".into());
        if let Some(incremental) = self.incremental {
            environment.insert(
                "CARGO_INCREMENTAL".into(),
                if incremental { "1" } else { "0" }.into(),
            );
        }
        let bindings = declarations
            .map(|declared| {
                declared.select(root, &rustup, &self.toolchain, configured, &environment)
            })
            .transpose()?;
        let version = |name: &str, selected: Option<&Executable>| -> Result<String, String> {
            let path = environment
                .get(OsStr::new("PATH"))
                .map(|v| v.to_str().ok_or("non UTF-8 PATH"))
                .transpose()?;
            let executable = match selected {
                Some(value) => PathBuf::from(&value.path),
                None => resolve_program(root, name, path)?,
            };
            bindings::output_text(
                bindings::capture(root, &executable, &["--version"], &environment)?,
                name,
            )
        };
        let versions = Tools {
            cargo: version("cargo", bindings.as_ref().map(|value| &value.cargo))?,
            rustc: version("rustc", bindings.as_ref().map(|value| &value.rustc))?,
        };
        let expected = format!("rustc {}", self.toolchain);
        if !policy::version_matches(versions.rustc.as_bytes(), expected.as_bytes(), false) {
            return Err("unexpected rustc version".into());
        }
        if let Some(selected) = &bindings {
            environment.insert("RUSTC".into(), selected.rustc.path.clone().into());
            environment.insert("RUSTC_WRAPPER".into(), "".into());
            environment.insert("RUSTC_WORKSPACE_WRAPPER".into(), "".into());
        }
        Ok(Selected {
            environment,
            versions,
            bindings,
        })
    }
}

/// 先解释绑定和全部环境，再允许安装；固定注册表字节由调用方提供。
pub fn select(
    root: &Path,
    config: &[u8],
    native: &NativePlatform,
    registry: Option<&[u8]>,
    inherited: &Environment,
    configured: &BTreeMap<String, String>,
) -> Result<Selected, Failure> {
    let declarations = Declarations::parse(config, native, registry)?;
    Prepared::parse(root, config, inherited)?.activate(root, declarations.as_ref(), configured)
}
