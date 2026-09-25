//! Explicit host instruction generation. No judge execution or host discovery.

mod transaction;

use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use transaction::{Change, Fault, Snapshot};

const DIRECTORY: &str = ".chrono-harness/instructions";
const MANIFEST: &str = ".chrono-harness/instructions/manifest.json";
const METHOD: &str = ".chrono-harness/instructions/methodology.md";
const CONTEXT: &str = ".chrono-harness/instructions/host-context.md";
const BEGIN: &[u8] = b"<!-- chrono-instructions:begin -->";
const END: &[u8] = b"<!-- chrono-instructions:end -->";
const RESERVED: &[u8] = b"<!-- chrono-instructions";
const ROUTE: &str = include_str!("../../assets/entrypoint.md");

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Source {
    role: String,
    path: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Registration {
    schema_version: u32,
    producer: String,
    render: String,
    sources: Vec<Source>,
    outputs: Vec<Source>,
}

fn registration() -> Registration {
    let source = |role: &str, path: &str| Source {
        role: role.into(),
        path: path.into(),
    };
    Registration {
        schema_version: 1,
        producer: concat!("chrono-instructions/", env!("CARGO_PKG_VERSION")).into(),
        render: "read-both/v1".into(),
        sources: vec![
            source("methodology", METHOD),
            source("host-context", CONTEXT),
        ],
        outputs: vec![
            source("agents-entrypoint", "AGENTS.md"),
            source("claude-entrypoint", "CLAUDE.md"),
        ],
    }
}

fn error(path: &Path, message: impl std::fmt::Display) -> String {
    format!("{}: {message}", path.display())
}

fn metadata(path: &Path) -> Result<Option<fs::Metadata>, String> {
    match fs::symlink_metadata(path) {
        Ok(meta) => Ok(Some(meta)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(error(path, e)),
    }
}

fn regular(path: &Path) -> Result<Option<Snapshot>, String> {
    let Some(meta) = metadata(path)? else {
        return Ok(None);
    };
    if !meta.file_type().is_file() {
        return Err(error(
            path,
            "expected a regular file (aliases are not supported here)",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if meta.nlink() != 1 {
            return Err(error(
                path,
                "hard-linked retained files/outputs are unsupported",
            ));
        }
    }
    Ok(Some(Snapshot {
        bytes: fs::read(path).map_err(|e| error(path, e))?,
        permissions: meta.permissions(),
    }))
}

fn utf8(path: &Path, bytes: &[u8]) -> Result<(), String> {
    std::str::from_utf8(bytes)
        .map(|_| ())
        .map_err(|e| error(path, format!("expected UTF-8: {e}")))
}

fn input(path: &Path) -> Result<Vec<u8>, String> {
    // Explicit external inputs may use aliases; retained host files may not.
    let meta = fs::metadata(path).map_err(|e| error(path, e))?;
    if !meta.is_file() {
        return Err(error(path, "expected a readable regular input file"));
    }
    let bytes = fs::read(path).map_err(|e| error(path, e))?;
    utf8(path, &bytes)?;
    Ok(bytes)
}

fn positions(bytes: &[u8], needle: &[u8]) -> Vec<usize> {
    bytes
        .windows(needle.len())
        .enumerate()
        .filter_map(|(i, part)| (part == needle).then_some(i))
        .collect()
}

fn line_end(bytes: &[u8], offset: usize) -> bool {
    offset == bytes.len()
        || bytes[offset..].starts_with(b"\n")
        || bytes[offset..].starts_with(b"\r\n")
}

fn render(path: &Path, bytes: &[u8]) -> Result<Vec<u8>, String> {
    let reserved = positions(bytes, RESERVED);
    let block = ROUTE.trim_end_matches('\n').as_bytes();
    if reserved.is_empty() {
        let mut result = bytes.to_vec();
        if !result.is_empty() && !result.ends_with(b"\n") {
            result.push(b'\n');
        }
        result.extend_from_slice(block);
        result.push(b'\n');
        return Ok(result);
    }
    let starts = positions(bytes, BEGIN);
    let ends = positions(bytes, END);
    if reserved.len() != 2 || starts.len() != 1 || ends.len() != 1 {
        return Err(error(path, "malformed or duplicate managed delimiters"));
    }
    let (start, end) = (starts[0], ends[0]);
    if start >= end
        || (start > 0 && bytes[start - 1] != b'\n')
        || (end > 0 && bytes[end - 1] != b'\n')
        || !line_end(bytes, start + BEGIN.len())
        || !line_end(bytes, end + END.len())
    {
        return Err(error(
            path,
            "managed delimiters must be ordered standalone lines",
        ));
    }
    let mut result = bytes[..start].to_vec();
    result.extend_from_slice(block);
    result.extend_from_slice(&bytes[end + END.len()..]);
    Ok(result)
}

fn prepare(
    host: &Path,
    selected: Option<(Vec<u8>, Vec<u8>)>,
) -> Result<(Vec<Change>, Vec<PathBuf>), String> {
    let root = fs::canonicalize(host).map_err(|e| error(host, e))?;
    if !root.is_dir() {
        return Err(error(&root, "host root must be an existing directory"));
    }
    let mut directories = Vec::new();
    for relative in [".chrono-harness", DIRECTORY] {
        let path = root.join(relative);
        match metadata(&path)? {
            Some(meta) if !meta.file_type().is_dir() => {
                return Err(error(
                    &path,
                    "expected a directory, not a symlink or other type",
                ));
            }
            None => directories.push(path),
            _ => {}
        }
    }
    let mut changes = Vec::new();
    let manifest_path = root.join(MANIFEST);
    let manifest = regular(&manifest_path)?;
    let method_path = root.join(METHOD);
    let context_path = root.join(CONTEXT);
    let method = regular(&method_path)?;
    let context = regular(&context_path)?;
    if let Some(existing) = manifest {
        let parsed: Registration = serde_json::from_slice(&existing.bytes)
            .map_err(|e| error(&manifest_path, format!("invalid registration: {e}")))?;
        if parsed != registration() {
            return Err(error(
                &manifest_path,
                "unsupported registration: schema, producer, render, ordered sources and output roles must match v1 exactly",
            ));
        }
        let method = method.ok_or_else(|| error(&method_path, "registered source is missing"))?;
        let context =
            context.ok_or_else(|| error(&context_path, "registered source is missing"))?;
        utf8(&method_path, &method.bytes)?;
        utf8(&context_path, &context.bytes)?;
        if let Some((new_method, new_context)) = selected {
            if method.bytes != new_method || context.bytes != new_context {
                return Err(error(
                    &manifest_path,
                    "init inputs differ from retained canonical sources; edit retained sources deliberately and use generate",
                ));
            }
        }
    } else {
        let (new_method, new_context) = selected.ok_or_else(|| {
            error(
                &manifest_path,
                "missing registration; run explicit init first",
            )
        })?;
        for (path, exists) in [
            (&method_path, method.is_some()),
            (&context_path, context.is_some()),
        ] {
            if exists {
                return Err(error(
                    path,
                    "reserved source collision without registration",
                ));
            }
        }
        changes.push(Change::new(method_path, None, new_method));
        changes.push(Change::new(context_path, None, new_context));
        let mut bytes = serde_json::to_vec_pretty(&registration()).map_err(|e| e.to_string())?;
        bytes.push(b'\n');
        changes.push(Change::new(manifest_path, None, bytes));
    }
    let agents = root.join("AGENTS.md");
    let claude = root.join("CLAUDE.md");
    let alias = metadata(&agents)?.is_some_and(|m| m.file_type().is_symlink());
    let claude_before = regular(&claude)?;
    if alias {
        if fs::read_link(&agents)
            .map_err(|e| error(&agents, e))?
            .as_os_str()
            != OsStr::new("CLAUDE.md")
            || claude_before.is_none()
        {
            return Err(error(
                &agents,
                "only the relative alias AGENTS.md -> CLAUDE.md with a regular existing target is supported",
            ));
        }
    } else {
        let before = regular(&agents)?;
        let after = render(&agents, before.as_ref().map_or(&[], |s| s.bytes.as_slice()))?;
        changes.push(Change::new(agents, before, after));
    }
    let after = render(
        &claude,
        claude_before.as_ref().map_or(&[], |s| s.bytes.as_slice()),
    )?;
    changes.push(Change::new(claude, claude_before, after));
    changes.retain(|c| c.before.as_ref().is_none_or(|s| s.bytes != c.after));
    Ok((changes, directories))
}

/// Retain exact selected UTF-8 bytes and generate both discovery routes.
/// Repeated init refuses differing retained sources. The host must already exist.
pub fn init(host: &Path, methodology: &Path, context: &Path) -> Result<Vec<PathBuf>, String> {
    let selected = (input(methodology)?, input(context)?);
    let (changes, directories) = prepare(host, Some(selected))?;
    transaction::publish(changes, directories, Fault::default())
}

/// Validate registered sources and refresh routes while preserving host-owned bytes.
pub fn generate(host: &Path) -> Result<Vec<PathBuf>, String> {
    let (changes, directories) = prepare(host, None)?;
    transaction::publish(changes, directories, Fault::default())
}

/// Bounded IO-failure injection available only in the dedicated test build.
#[cfg(feature = "test-support")]
pub mod test_support {
    use super::*;

    /// Fail before publication `after` (zero based); optionally fail one rollback.
    /// No production CLI or environment switch enables this hook.
    pub fn init_with_failure(
        host: &Path,
        method: &Path,
        context: &Path,
        after: usize,
        rollback: Option<usize>,
    ) -> Result<Vec<PathBuf>, String> {
        let (changes, directories) = prepare(host, Some((input(method)?, input(context)?)))?;
        transaction::publish(
            changes,
            directories,
            Fault {
                after: Some(after),
                rollback,
            },
        )
    }
}

/// CLI transport result; exit 0 complete, 2 usage, 1 input/generation/IO failure.
#[derive(Debug)]
pub struct CliOutput {
    pub exit_code: u8,
    pub stdout: String,
    pub stderr: String,
}

/// Accept OS paths without shell interpolation or UTF-8 conversion of arguments.
pub fn dispatch(args: &[OsString]) -> CliOutput {
    let reply = |exit_code, stdout: String, stderr: String| CliOutput {
        exit_code,
        stdout,
        stderr,
    };
    if args.is_empty()
        || (args.len() == 1 && ["help", "--help", "-h"].iter().any(|a| args[0] == *a))
    {
        return reply(0, "Usage:\n  chrono-instructions init --host-root H --methodology M --host-context C\n  chrono-instructions generate --host-root H\nExplicit paths; H must exist. C may be empty. Exit 0 complete, 2 usage, 1 generation/IO failure. No judges run.\n".into(), String::new());
    }
    if args.len() == 1 && args[0] == "--version" {
        return reply(
            0,
            concat!("chrono-instructions ", env!("CARGO_PKG_VERSION"), "\n").into(),
            String::new(),
        );
    }
    let usage = || {
        reply(
            2,
            String::new(),
            "E_USAGE: use --help; required options must occur exactly once.\n".into(),
        )
    };
    let is_init = args[0] == "init";
    if !is_init && args[0] != "generate" {
        return usage();
    }
    let mut host = None;
    let mut method = None;
    let mut context = None;
    let mut parts = args[1..].chunks_exact(2);
    for pair in &mut parts {
        let slot = if pair[0] == "--host-root" {
            &mut host
        } else if is_init && pair[0] == "--methodology" {
            &mut method
        } else if is_init && pair[0] == "--host-context" {
            &mut context
        } else {
            return usage();
        };
        if slot.is_some() || pair[1].is_empty() {
            return usage();
        }
        *slot = Some(PathBuf::from(&pair[1]));
    }
    if !parts.remainder().is_empty() {
        return usage();
    }
    let Some(host) = host else {
        return usage();
    };
    let result = if is_init {
        let (Some(method), Some(context)) = (method, context) else {
            return usage();
        };
        init(&host, &method, &context)
    } else {
        generate(&host)
    };
    match result {
        Ok(paths) => reply(
            0,
            format!(
                "complete: {} file(s) changed; no judges executed\n",
                paths.len()
            ),
            String::new(),
        ),
        Err(e) => reply(1, String::new(), format!("E_GENERATION: {e}\n")),
    }
}
