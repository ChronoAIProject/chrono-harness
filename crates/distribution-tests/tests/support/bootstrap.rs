//! Finite boundary data for the two generated distribution launcher profiles.
//! The template digest is independent fixture input: a changed shell body must
//! explicitly revise this model and its expectations. No Shell is interpreted.
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::{verified_identifiers as identifiers, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
verus! {
pub fn known_mode(mode:&str) -> (known:bool)
    ensures known == (mode@ == "install"@ || mode@ == "adopt"@),
{ identifiers::equal_text(mode,"install") || identifiers::equal_text(mode,"adopt") }
}
#[cfg(not(any(verus_keep_ghost, test)))]
mod runtime {
    use std::{
        collections::BTreeMap,
        env, fs,
        io::{self, Read},
        os::unix::fs::PermissionsExt,
        path::{Path, PathBuf},
        process::Command,
    };
    const TEMPLATE: &str = include_str!("../../../../assets/distribution/bootstrap.sh");
    fn unquote(value: &str) -> Result<String, String> {
        let fields = shlex::split(value).ok_or("invalid launcher declaration quote")?;
        if fields.len() != 1 || format!("'{}'", fields[0].replace('\'', "'\"'\"'")) != value {
            return Err("unsupported launcher declaration quote".into());
        }
        Ok(fields[0].clone())
    }
    fn profile(
        script: &str,
    ) -> Result<(BTreeMap<String, String>, BTreeMap<String, (String, u64)>), String> {
        let domain: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/bootstrap-domain.json")).unwrap();
        if domain["template_sha256"].as_str()
            != Some(chrono_harness::sha256(TEMPLATE.as_bytes()).as_str())
        {
            return Err("unregistered launcher template".into());
        }
        let (prefix, rest) = TEMPLATE.split_once("@DECLARATIONS@").unwrap();
        let (middle, suffix) = rest.split_once("@INSTALLERS@").unwrap();
        let body = script
            .strip_prefix(prefix)
            .ok_or("unknown launcher prefix")?;
        let (raw_declarations, body) = body.split_once(middle).ok_or("unknown launcher body")?;
        let raw_installers = body.strip_suffix(suffix).ok_or("unknown launcher suffix")?;
        let mut declarations = BTreeMap::new();
        for line in raw_declarations.lines() {
            let (name, value) = line
                .strip_prefix("    ")
                .and_then(|line| line.split_once('='))
                .ok_or("invalid launcher declaration")?;
            if ![
                "mode",
                "version",
                "source_kind",
                "source_location",
                "curl",
                "default_tools",
                "manifest_sha256",
                "manifest_size",
                "lock_sha256",
                "lock_size",
            ]
            .contains(&name)
            {
                return Err(format!("unknown launcher declaration {name}"));
            }
            let value = if name == "lock_size" {
                value.parse::<u64>().map_err(|e| e.to_string())?.to_string()
            } else {
                unquote(value)?
            };
            if declarations.insert(name.to_owned(), value).is_some() {
                return Err("duplicate launcher declaration".into());
            }
        }
        for name in [
            "mode",
            "version",
            "source_kind",
            "source_location",
            "curl",
            "default_tools",
            "manifest_sha256",
            "manifest_size",
        ] {
            if !declarations.contains_key(name) {
                return Err(format!("missing launcher declaration {name}"));
            }
        }
        if !super::known_mode(&declarations["mode"]) {
            return Err("unknown launcher mode".into());
        }
        if declarations["mode"] == "install"
            && !(declarations.contains_key("lock_sha256") && declarations.contains_key("lock_size"))
        {
            return Err("missing launcher lock binding".into());
        }
        let mut installers = BTreeMap::new();
        for line in raw_installers.lines() {
            let (platform, rest) = line
                .strip_prefix("        ")
                .and_then(|line| line.split_once(") installer_sha256="))
                .ok_or("unknown launcher platform entry")?;
            let (digest, size) = rest
                .strip_suffix(" ;;")
                .and_then(|text| text.split_once("; installer_size="))
                .ok_or("unknown launcher installer binding")?;
            let platform = unquote(platform)?;
            if ![
                "macos-aarch64",
                "macos-x86_64",
                "linux-aarch64",
                "linux-x86_64",
            ]
            .contains(&platform.as_str())
            {
                return Err("unknown launcher platform".into());
            }
            if installers
                .insert(
                    platform,
                    (
                        unquote(digest)?,
                        size.parse::<u64>().map_err(|e| e.to_string())?,
                    ),
                )
                .is_some()
            {
                return Err("duplicate launcher platform".into());
            }
        }
        Ok((declarations, installers))
    }
    fn verify(path: &Path, digest: &str, size: u64) -> Result<(), String> {
        let bytes = fs::read(path).map_err(|_| format!("missing file: {}", path.display()))?;
        if bytes.len() as u64 != size || chrono_harness::sha256(&bytes) != digest {
            return Err(format!("integrity mismatch: {}", path.display()));
        }
        Ok(())
    }
    fn fetch(
        profile: &BTreeMap<String, String>,
        member: &str,
        destination: &Path,
    ) -> Result<Option<i32>, String> {
        match profile["source_kind"].as_str() {
            "directory" => {
                fs::copy(
                    Path::new(&profile["source_location"]).join(member),
                    destination,
                )
                .map_err(|e| e.to_string())?;
                Ok(None)
            }
            "https" => {
                let bound = PathBuf::from(
                    env::var_os("CHRONO_FIXTURE_CURL")
                        .ok_or("missing fixture transport binding")?,
                );
                if Path::new(&profile["curl"]) != bound {
                    return Err("unbound launcher transport".into());
                }
                let status = Command::new(bound)
                    .args([
                        "--disable",
                        "--fail",
                        "--location",
                        "--silent",
                        "--show-error",
                        "--proto",
                        "=https",
                        "--proto-redir",
                        "=https",
                        "--connect-timeout",
                        "30",
                        "--max-time",
                        "300",
                        "--output",
                    ])
                    .arg(destination)
                    .arg("--url")
                    .arg(format!(
                        "{}/{}",
                        profile["source_location"].trim_end_matches('/'),
                        member
                    ))
                    .status()
                    .map_err(|e| e.to_string())?;
                if status.success() {
                    Ok(None)
                } else {
                    Ok(Some(
                        status
                            .code()
                            .ok_or("fixture transport has no original exit code")?,
                    ))
                }
            }
            _ => Err("unknown launcher source".into()),
        }
    }
    fn installer(program: &Path, args: &[String]) -> Result<i32, String> {
        let bytes = fs::read(program).map_err(|e| e.to_string())?;
        for (sample, code) in [
            (include_bytes!("../fixtures/installer-ok.sh").as_slice(), 0),
            (
                include_bytes!("../fixtures/installer-exit-23.sh").as_slice(),
                23,
            ),
        ] {
            if bytes == sample {
                return Ok(code);
            }
        }
        let bound =
            env::var_os("CHRONO_FIXTURE_INSTALLER").ok_or("missing fixture installer binding")?;
        if bytes != fs::read(bound).map_err(|e| e.to_string())? {
            return Err("unknown installer fixture bytes".into());
        }
        let status = Command::new(program)
            .args(args)
            .status()
            .map_err(|e| e.to_string())?;
        status
            .code()
            .ok_or("fixture installer has no original exit code".into())
    }
    fn run(args: &[String]) -> Result<i32, String> {
        let (script, arguments) = if args.get(0).map(String::as_str) == Some("-s")
            && args.get(1).map(String::as_str) == Some("--")
        {
            let mut script = String::new();
            io::stdin()
                .read_to_string(&mut script)
                .map_err(|e| e.to_string())?;
            (script, &args[2..])
        } else {
            let path = args.first().ok_or("missing launcher script")?;
            (
                fs::read_to_string(path).map_err(|e| e.to_string())?,
                &args[1..],
            )
        };
        let (profile, installers) = profile(&script)?;
        let adopt = profile["mode"] == "adopt";
        let mut root = PathBuf::from(".");
        let mut host_seen = false;
        let mut tools = Vec::new();
        let mut replace = false;
        let mut at = 0;
        while at < arguments.len() {
            match arguments[at].as_str() {
                "--host-root" => {
                    if host_seen {
                        return Err("duplicate --host-root".into());
                    }
                    host_seen = true;
                    let value = arguments
                        .get(at + 1)
                        .filter(|value| !value.is_empty())
                        .ok_or("missing --host-root value")?;
                    root = PathBuf::from(value);
                    at += 2;
                }
                "--tool" if adopt => {
                    let value = arguments
                        .get(at + 1)
                        .filter(|value| !value.is_empty())
                        .ok_or("missing --tool value")?;
                    if value == "."
                        || value == ".."
                        || !value
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
                    {
                        return Err("invalid tool name".into());
                    }
                    tools.push(value.clone());
                    at += 2;
                }
                "--replace" if adopt && !replace => {
                    replace = true;
                    at += 1;
                }
                unknown => return Err(format!("unknown option: {unknown}")),
            }
        }
        let root = fs::canonicalize(root).map_err(|_| "host root must be an existing directory")?;
        let platform = chrono_distribution::platform();
        let (digest, size) = installers
            .get(&platform)
            .ok_or("release has no fixture platform installer")?;
        if !adopt {
            verify(
                &root.join(".chrono-harness/distribution.json"),
                &profile["lock_sha256"],
                profile["lock_size"]
                    .parse::<u64>()
                    .map_err(|e| e.to_string())?,
            )?;
        }
        let temporary = tempfile::Builder::new()
            .prefix("chrono-install.")
            .tempdir_in(env::var_os("TMPDIR").ok_or("private launcher temporary root required")?)
            .map_err(|e| e.to_string())?;
        let program = temporary.path().join("chrono-distribution");
        if let Some(code) = fetch(
            &profile,
            &format!("chrono-distribution-{platform}"),
            &program,
        )? {
            return Ok(code);
        }
        verify(&program, digest, *size)?;
        fs::set_permissions(&program, fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
        if adopt {
            let manifest = temporary.path().join("release.json");
            if let Some(code) = fetch(&profile, "release.json", &manifest)? {
                return Ok(code);
            }
            verify(
                &manifest,
                &profile["manifest_sha256"],
                profile["manifest_size"]
                    .parse::<u64>()
                    .map_err(|e| e.to_string())?,
            )?;
            let mut argv = vec![
                "adopt".into(),
                "--host-root".into(),
                root.to_str().ok_or("host path encoding")?.into(),
                "--manifest".into(),
                manifest.to_str().unwrap().into(),
            ];
            if profile["source_kind"] == "directory" {
                argv.extend(["--source-dir".into(), profile["source_location"].clone()]);
            } else {
                argv.extend([
                    "--base-url".into(),
                    profile["source_location"].clone(),
                    "--curl".into(),
                    profile["curl"].clone(),
                ]);
            }
            if replace {
                argv.push("--replace".into());
            }
            if tools.is_empty() {
                tools.extend(
                    profile["default_tools"]
                        .split_whitespace()
                        .map(str::to_owned),
                );
            }
            for tool in tools {
                argv.extend(["--tool".into(), tool]);
            }
            let code = installer(&program, &argv)?;
            if code != 0 {
                return Ok(code);
            }
        }
        installer(
            &program,
            &[
                "install".into(),
                "--host-root".into(),
                root.to_str().unwrap().into(),
            ],
        )
    }
    pub fn dispatch(args: &[String]) -> i32 {
        match run(args) {
            Ok(code) => code,
            Err(error) => {
                eprintln!("E_DISTRIBUTION_BOOTSTRAP: {error}");
                1
            }
        }
    }
}
#[cfg(not(any(verus_keep_ghost, test)))]
pub use runtime::dispatch;
