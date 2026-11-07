//! 工具绑定的 JSON 和原生 IO 适配；身份与选择判断复用同源证明内核。
use crate::verified_bindings::{self as policy, ConfiguredAction, DeclarationSource, Executable};
use chrono_harness::{file_identity, json, process_fds, resolve_program};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

pub type Environment = BTreeMap<OsString, OsString>;
pub use policy::NativePlatform;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub path: Option<String>,
    pub sha256: String,
    pub version: String,
    #[serde(default)]
    pub allow_relocation: bool,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Tools<T> {
    pub cargo: T,
    pub rustc: T,
}

pub struct Declarations {
    pub tools: Tools<Binding>,
    registry: bool,
}

fn typed<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, String> {
    serde_json::from_value(value.clone()).map_err(|error| error.to_string())
}

impl Declarations {
    /// 注册表字节由当前源码读取方提供，历史端点仍只读取固定数据。
    pub fn parse(
        config_bytes: &[u8],
        native: &NativePlatform,
        registry_bytes: Option<&[u8]>,
    ) -> Result<Option<Self>, String> {
        let config = json(config_bytes)?;
        let required = config
            .get("require_tool_bindings")
            .map(typed::<bool>)
            .transpose()?
            .unwrap_or(false);
        let platform = config
            .get("native_platform")
            .map(typed::<NativePlatform>)
            .transpose()?;
        let platform_matches = policy::platform_matches(platform.as_ref(), native);
        let source = policy::declaration_source(
            config.get("tool_bindings_registry").is_some(),
            config.get("tool_bindings").is_some(),
            config.get("native_platform").is_some(),
            required,
            platform_matches,
        );
        let registry = matches!(source, DeclarationSource::Registry);
        let tools = match source {
            DeclarationSource::Reject => {
                return Err("conflicting or missing native bootstrap binding declarations".into());
            }
            DeclarationSource::Unbound => return Ok(None),
            DeclarationSource::Direct => typed::<Tools<Binding>>(&config["tool_bindings"])?,
            DeclarationSource::Registry => {
                let _: String = typed(&config["tool_bindings_registry"])?;
                let bytes =
                    registry_bytes.ok_or("bootstrap toolchain registry bytes are missing")?;
                let registered = json(bytes)?;
                let schema: String = typed(&registered["schema"])?;
                let toolchain: String = typed(&registered["rust_toolchain"])?;
                let expected: String = typed(&config["rust_toolchain"])?;
                if !policy::registry_matches(&schema, &toolchain, &expected) {
                    return Err("bootstrap toolchain registry differs".into());
                }
                #[derive(Deserialize)]
                struct RegistryTool {
                    sha256: String,
                    version: String,
                }
                let key = format!("{}/{}", native.system, native.machine);
                let original: Tools<RegistryTool> = typed(&registered["platforms"][&key]["tools"])?;
                let binding = |tool: RegistryTool| Binding {
                    path: None,
                    sha256: tool.sha256,
                    version: tool.version,
                    allow_relocation: true,
                };
                Tools {
                    cargo: binding(original.cargo),
                    rustc: binding(original.rustc),
                }
            }
        };
        for (name, binding) in [("cargo", &tools.cargo), ("rustc", &tools.rustc)] {
            if !policy::binding_valid(binding.sha256.as_bytes(), registry, binding.path.is_some()) {
                return Err(format!("invalid {name} executable binding"));
            }
        }
        Ok(Some(Self { tools, registry }))
    }

    /// 原命令只读查询 Rustup；先核选中文件摘要，再允许执行版本查询。
    pub fn select(
        &self,
        root: &Path,
        rustup: &Path,
        toolchain: &str,
        configured: &BTreeMap<String, String>,
        environment: &Environment,
    ) -> Result<Tools<Executable>, String> {
        let observe = |name: &str, binding: &Binding| -> Result<Executable, String> {
            let output = capture(
                root,
                rustup,
                &["which", "--toolchain", toolchain, name],
                environment,
            )?;
            let selected_path = PathBuf::from(output_text(output, "rustup which")?);
            if !selected_path.is_absolute() {
                return Err(format!("rustup selected non-absolute {name} path"));
            }
            physical_file(&selected_path)?;
            if !chrono_harness::verified_identifiers::equal_text(
                &file_identity(&selected_path)?.0,
                &binding.sha256,
            ) {
                return Err(format!("bound {name} identity mismatch"));
            }
            let selected = executable(root, &selected_path, name, environment)?;
            let expected = Executable {
                path: binding
                    .path
                    .as_ref()
                    .map(|path| root.join(path))
                    .unwrap_or_default()
                    .to_str()
                    .ok_or("non UTF-8 executable path")?
                    .to_owned(),
                sha256: binding.sha256.clone(),
                version: binding.version.clone(),
            };
            if !policy::selected_matches(&selected, &expected, binding.allow_relocation) {
                return Err(format!("bound {name} identity or relocation mismatch"));
            }
            match policy::configured_action(
                configured.get(name).map(String::as_str),
                binding.path.as_deref(),
                binding.allow_relocation,
                self.registry,
            ) {
                ConfiguredAction::Missing => {
                    return Err(format!("tool_bindings requires configured {name}"));
                }
                ConfiguredAction::Observe => {
                    let path = environment
                        .get(OsStr::new("PATH"))
                        .map(|value| value.to_str().ok_or("non UTF-8 PATH"))
                        .transpose()?;
                    let configured_path = resolve_program(root, &configured[name], path)?;
                    let actual = executable(root, &configured_path, name, environment)?;
                    if !policy::payload_equal(&actual, &selected) {
                        return Err(format!("configured {name} selection differs from binding"));
                    }
                }
                ConfiguredAction::Retired | ConfiguredAction::NotDeclared => {}
            }
            let prefix = format!("{name} {toolchain}");
            if !policy::version_matches(
                selected.version.as_bytes(),
                prefix.as_bytes(),
                name == "cargo",
            ) {
                return Err(format!("{name} toolchain selection mismatch"));
            }
            Ok(selected)
        };
        let cargo = observe("cargo", &self.tools.cargo)?;
        let rustc = observe("rustc", &self.tools.rustc)?;
        let compiler = environment
            .get(OsStr::new("RUSTC"))
            .filter(|value| !value.is_empty())
            .map(|value| fs::canonicalize(root.join(value)).map_err(|error| error.to_string()))
            .transpose()?;
        let inherited = compiler
            .as_ref()
            .map(|value| value.to_str().ok_or("non UTF-8 inherited compiler"))
            .transpose()?;
        let selected_compiler = fs::canonicalize(&rustc.path).map_err(|error| error.to_string())?;
        let wrapper = |key| {
            environment
                .get(OsStr::new(key))
                .map(|value| value.to_str().ok_or("non UTF-8 compiler wrapper"))
                .transpose()
        };
        if !policy::compiler_selection(
            selected_compiler
                .to_str()
                .ok_or("non UTF-8 compiler path")?,
            inherited,
            wrapper("RUSTC_WRAPPER")?,
            wrapper("RUSTC_WORKSPACE_WRAPPER")?,
        ) {
            return Err("inherited compiler selection or wrapper differs from binding".into());
        }
        Ok(Tools { cargo, rustc })
    }
}

fn physical_file(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if !metadata.file_type().is_file() {
        return Err(format!(
            "bound executable is not a physical regular file: {}",
            path.display()
        ));
    }
    Ok(())
}

fn executable(
    root: &Path,
    path: &Path,
    name: &str,
    environment: &Environment,
) -> Result<Executable, String> {
    physical_file(path)?;
    let version = output_text(capture(root, path, &["--version"], environment)?, name)?;
    let prefix = format!("{name} ");
    if !policy::byte_prefix(version.as_bytes(), prefix.as_bytes()) {
        return Err(format!("bound {name} version mismatch"));
    }
    Ok(Executable {
        path: path.to_str().ok_or("non UTF-8 executable path")?.to_owned(),
        sha256: file_identity(path)?.0,
        version,
    })
}

fn capture(
    root: &Path,
    program: &Path,
    args: &[&str],
    environment: &Environment,
) -> Result<Output, String> {
    let mut command = Command::new(program);
    command.current_dir(root).args(args).env_clear();
    // 租约载体由现有所有者产生，不能把继承值当成新命令的显式绑定。
    command.envs(
        environment
            .iter()
            .filter(|(key, _)| key.as_os_str() != OsStr::new(process_fds::ENV)),
    );
    process_fds::forward_command(&mut command, &[])?;
    command.output().map_err(|error| error.to_string())
}

fn output_text(output: Output, name: &str) -> Result<String, String> {
    if !output.status.success() {
        return Err(format!(
            "{name} failed: {}; stderr: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout)
        .map(|text| text.trim().to_owned())
        .map_err(|error| error.to_string())
}
