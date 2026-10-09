//! Explicit host instruction generation. No judge execution or host discovery.

mod catalog;
mod transaction;

use catalog::{Atom, Catalog, Content, Format, Manifest, Variant};
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use transaction::{Change, Fault, Snapshot};

const MANIFEST: &str = ".chrono-harness/instructions/manifest.json";
const METHOD: &str = ".chrono-harness/instructions/methodology.md";
const CONTEXT: &str = ".chrono-harness/instructions/host-context.md";
const BEGIN: &[u8] = b"<!-- chrono-instructions:begin -->";
const END: &[u8] = b"<!-- chrono-instructions:end -->";
const RESERVED: &[u8] = b"<!-- chrono-instructions";
const CATALOG: &str = ".chrono-harness/instructions/catalog.json";

struct InitSources {
    methodology: Option<Vec<u8>>,
    context: Option<Vec<u8>>,
    locale: Option<String>,
}

impl InitSources {
    fn read(methodology: Option<&Path>, context: Option<&Path>) -> Result<Self, String> {
        Ok(Self {
            methodology: methodology.map(input).transpose()?,
            context: context.map(input).transpose()?,
            locale: None,
        })
    }
}

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
        producer: "chrono-instructions/0.1.0".into(),
        render: "literal-core/relative-alias/v2".into(),
        sources: vec![
            source("methodology", METHOD),
            source("host-context", CONTEXT),
        ],
        outputs: vec![
            source("claude-guide", "CLAUDE.md"),
            source("agents-relative-alias", "AGENTS.md"),
        ],
    }
}

// Only this exact earlier identity is a forward migration input.
fn legacy_registration() -> Registration {
    let mut old = registration();
    old.producer = "chrono-instructions/0.1.0".into();
    old.render = "read-both/v1".into();
    old.outputs = vec![
        Source {
            role: "agents-entrypoint".into(),
            path: "AGENTS.md".into(),
        },
        Source {
            role: "claude-entrypoint".into(),
            path: "CLAUDE.md".into(),
        },
    ];
    old
}

fn managed_block(manifest: &Manifest, frame: &str, body: &str) -> Vec<u8> {
    let mut block = BEGIN.to_vec();
    block.extend_from_slice(
        format!(
            "\n{frame}\n\nmanifest: `{MANIFEST}`\ncatalog: `{}`\nhost_context: `{}`\n\n{}\n",
            manifest.catalog,
            manifest.host_context,
            manifest.root().title_body(body)
        )
        .as_bytes(),
    );
    block.extend_from_slice(END);
    block
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

fn render(path: &Path, bytes: &[u8], block: &[u8]) -> Result<Vec<u8>, String> {
    let reserved = positions(bytes, RESERVED);
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

// Canonical lexical paths keep the registry unambiguous; existing ancestors are
// checked separately before reading or writing any registered file.
fn relative(path: &str, source: bool) -> Result<(), String> {
    catalog::content_valid(path)?;
    if path.is_empty()
        || path.contains('\\')
        || path.chars().any(char::is_control)
        || path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        || Path::new(path).is_absolute()
    {
        return Err(format!("{path:?}: expected canonical relative path"));
    }
    if source && !path.starts_with(".chrono-harness/") {
        return Err(format!(
            "{path}: registered inputs must be under .chrono-harness/"
        ));
    }
    Ok(())
}

// Use Unicode case folding, then the platform's filesystem representation for
// prospective macOS paths. Lowercasing misses aliases such as straße / STRASSE.
// This conservatively rejects ambiguous spellings even on case-sensitive volumes;
// it does not change registered paths or probe the filesystem with writes.
#[cfg(target_os = "macos")]
fn mac_path_key(path: &str) -> Result<Vec<u8>, String> {
    use std::ffi::{c_char, c_void};
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringCreateWithBytes(
            allocator: *const c_void,
            bytes: *const u8,
            count: isize,
            encoding: u32,
            external: u8,
        ) -> *const c_void;
        fn CFStringCreateMutableCopy(
            allocator: *const c_void,
            capacity: isize,
            string: *const c_void,
        ) -> *mut c_void;
        fn CFStringFold(string: *mut c_void, flags: usize, locale: *const c_void);
        fn CFStringGetMaximumSizeOfFileSystemRepresentation(string: *const c_void) -> isize;
        fn CFStringGetFileSystemRepresentation(
            string: *const c_void,
            buffer: *mut c_char,
            size: isize,
        ) -> u8;
        fn CFRelease(value: *const c_void);
    }
    let count = isize::try_from(path.len()).map_err(|_| format!("{path}: path too long"))?;
    // SAFETY: create from a live UTF-8 slice using the SDK's UTF-8 encoding
    // constant. Fold only a mutable copy, using kCFCompareCaseInsensitive (1)
    // and the canonical system locale (NULL), not the user's locale. Check the
    // allocations and buffer bound; release both owned CF strings on every
    // return path. CF never retains the supplied byte buffer.
    unsafe {
        let original =
            CFStringCreateWithBytes(std::ptr::null(), path.as_ptr(), count, 0x08000100, 0);
        if original.is_null() {
            return Err(format!(
                "{path}: cannot create macOS path comparison string"
            ));
        }
        let string = CFStringCreateMutableCopy(std::ptr::null(), 0, original);
        CFRelease(original);
        if string.is_null() {
            return Err(format!("{path}: cannot copy macOS path comparison string"));
        }
        CFStringFold(string, 1, std::ptr::null());
        let size = CFStringGetMaximumSizeOfFileSystemRepresentation(string);
        if size <= 0 {
            CFRelease(string);
            return Err(format!("{path}: cannot size macOS path comparison string"));
        }
        let mut bytes = vec![0u8; size as usize];
        let converted =
            CFStringGetFileSystemRepresentation(string, bytes.as_mut_ptr().cast(), size);
        CFRelease(string);
        if converted == 0 {
            return Err(format!("{path}: cannot encode macOS filesystem path"));
        }
        let end = bytes
            .iter()
            .position(|b| *b == 0)
            .ok_or_else(|| format!("{path}: missing filesystem string terminator"))?;
        bytes.truncate(end);
        Ok(bytes)
    }
}

fn parents(root: &Path, relative: &str, directories: &mut BTreeSet<PathBuf>) -> Result<(), String> {
    let mut path = root.to_path_buf();
    let parts: Vec<_> = relative.split('/').collect();
    for part in &parts[..parts.len() - 1] {
        path.push(part);
        match metadata(&path)? {
            Some(meta) if !meta.file_type().is_dir() => {
                return Err(error(
                    &path,
                    "expected a directory, not a symlink or other type",
                ));
            }
            None => {
                directories.insert(path.clone());
            }
            _ => {}
        }
    }
    Ok(())
}

fn required(root: &Path, path: &str) -> Result<Vec<u8>, String> {
    let full = root.join(path);
    let snapshot = regular(&full)?.ok_or_else(|| error(&full, "registered source is missing"))?;
    utf8(&full, &snapshot.bytes)?;
    Ok(snapshot.bytes)
}

fn raw_catalog(locale: &str) -> Result<Catalog, String> {
    let mut cat: Catalog = catalog::parse(catalog::DEFAULT_CATALOG.as_bytes())?;
    if !cat.locales.iter().any(|l| l.id == locale) {
        return Err(format!("unregistered locale {locale}"));
    }
    cat.atoms = vec![Atom {
        id: "legacy.method".into(),
        requires: vec![],
        variants: vec![Variant {
            locale: locale.into(),
            source: Content::File {
                path: METHOD.into(),
            },
        }],
    }];
    cat.layouts.clear();
    Ok(cat)
}
fn raw_manifest(locale: &str) -> Result<Manifest, String> {
    let mut manifest: Manifest = catalog::parse(catalog::DEFAULT_MANIFEST.as_bytes())?;
    let output = &mut manifest.outputs[0];
    output.locale = locale.into();
    output.roots = vec!["legacy.method".into()];
    output.title = None;
    output.layout = None;
    Ok(manifest)
}

fn prepare(
    host: &Path,
    selected: Option<InitSources>,
) -> Result<(Vec<Change>, Vec<PathBuf>), String> {
    if !cfg!(unix) {
        return Err(
            "root guide publication requires Unix symlinks; this platform is unsupported".into(),
        );
    }
    let root = fs::canonicalize(host).map_err(|e| error(host, e))?;
    if !root.is_dir() {
        return Err(error(&root, "host root must be an existing directory"));
    }
    let mut directories = BTreeSet::new();
    parents(&root, MANIFEST, &mut directories)?;
    let manifest_path = root.join(MANIFEST);
    let manifest_before = regular(&manifest_path)?;
    let mut changes = Vec::new();
    let mut pending = BTreeMap::<String, Vec<u8>>::new();
    let mut new_manifest = false;
    let manifest: Manifest;
    if let Some(existing) = &manifest_before {
        match catalog::parse::<Manifest>(&existing.bytes) {
            Ok(parsed) => {
                parsed.validate().map_err(|e| error(&manifest_path, e))?;
                manifest = parsed;
            }
            Err(current_error) => {
                let parsed: Registration = catalog::parse(&existing.bytes).map_err(|e| {
                    error(
                        &manifest_path,
                        format!("invalid registration: {current_error}; legacy: {e}"),
                    )
                })?;
                if parsed != registration() && parsed != legacy_registration() {
                    return Err(error(
                        &manifest_path,
                        "unsupported registration: expected exact known v1/v2 migration identity",
                    ));
                }
                // Retain old method/context files; only register an opaque reference.
                if regular(&root.join(CATALOG))?.is_some() {
                    return Err(error(
                        &root.join(CATALOG),
                        "reserved source collision during migration",
                    ));
                }
                manifest = raw_manifest("und")?;
                pending.insert(CATALOG.into(), catalog::bytes(&raw_catalog("und")?)?);
                new_manifest = true;
            }
        }
    } else {
        let sources = selected.as_ref().ok_or_else(|| {
            error(
                &manifest_path,
                "missing registration; run explicit init first",
            )
        })?;
        for path in [METHOD, CONTEXT, CATALOG] {
            if regular(&root.join(path))?.is_some() {
                return Err(error(
                    &root.join(path),
                    "reserved source collision without registration",
                ));
            }
        }
        let (catalog_bytes, initial) = if let Some(method) = &sources.methodology {
            let locale = sources.locale.as_deref().unwrap_or("und");
            pending.insert(METHOD.into(), method.clone());
            (
                catalog::bytes(&raw_catalog(locale)?)?,
                raw_manifest(locale)?,
            )
        } else {
            let mut initial: Manifest = catalog::parse(catalog::DEFAULT_MANIFEST.as_bytes())?;
            if let Some(locale) = &sources.locale {
                if initial.outputs[0].locale != *locale {
                    initial.outputs[0].title = None;
                }
                initial.outputs[0].locale = locale.clone();
            }
            (catalog::DEFAULT_CATALOG.as_bytes().to_vec(), initial)
        };
        manifest = initial;
        pending.insert(CATALOG.into(), catalog_bytes);
        pending.insert(CONTEXT.into(), sources.context.clone().unwrap_or_default());
        new_manifest = true;
    }
    manifest.validate().map_err(|e| error(&manifest_path, e))?;
    for path in [&manifest.catalog, &manifest.host_context] {
        relative(path, true)?;
        parents(&root, path, &mut directories)?;
    }
    if manifest.catalog == manifest.host_context
        || manifest.catalog == MANIFEST
        || manifest.host_context == MANIFEST
    {
        return Err("source/control overlap in registration".into());
    }
    let catalog_bytes = match pending.get(&manifest.catalog) {
        Some(bytes) => bytes.clone(),
        None => required(&root, &manifest.catalog)?,
    };
    let cat: Catalog = catalog::parse(&catalog_bytes).map_err(|e| {
        error(
            &root.join(&manifest.catalog),
            format!("invalid catalog: {e}"),
        )
    })?;
    cat.validate()
        .map_err(|e| error(&root.join(&manifest.catalog), e))?;
    // Distinct control files and every output own one path. File variants may
    // explicitly share an input, but cannot alias a control file or output.
    let mut roles = BTreeMap::new();
    for (path, role) in [
        (MANIFEST, "manifest"),
        (&manifest.catalog, "catalog"),
        (&manifest.host_context, "host context"),
        ("AGENTS.md", "root alias"),
    ] {
        if roles.insert(path.to_owned(), role).is_some() {
            return Err(format!("{path}: source/control/alias overlap"));
        }
    }
    let file_inputs: BTreeSet<_> = cat.file_inputs().into_iter().collect();
    for path in &file_inputs {
        relative(path, true)?;
        if roles.insert((*path).to_owned(), "file source").is_some() {
            return Err(format!("{path}: source/control overlap"));
        }
    }
    for output in &manifest.outputs {
        relative(&output.path, false)?;
        if roles.insert(output.path.clone(), "output").is_some() {
            return Err(format!("{}: source/output/alias overlap", output.path));
        }
    }
    // macOS commonly uses case-insensitive volumes. Refuse inconsistent case
    // or normalization spellings, including prospective directories, even on a case-sensitive
    // macOS volume rather than publishing two ordinary aliases as two owners.
    #[cfg(target_os = "macos")]
    {
        let mut spellings = BTreeMap::new();
        for path in roles.keys() {
            let mut prefix = String::new();
            for component in path.split('/') {
                if !prefix.is_empty() {
                    prefix.push('/');
                }
                prefix.push_str(component);
                if let Some(previous) = spellings.insert(mac_path_key(&prefix)?, prefix.clone()) {
                    if previous != prefix {
                        return Err(format!(
                            "case alias or normalization alias in registered paths: {previous} and {prefix}"
                        ));
                    }
                }
            }
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let mut identities = BTreeMap::new();
        for path in roles.keys() {
            if let Some(meta) = metadata(&root.join(path))? {
                if meta.file_type().is_file() {
                    if let Some(previous) = identities.insert((meta.dev(), meta.ino()), path) {
                        return Err(format!("actual file alias: {previous} and {path}"));
                    }
                }
            }
        }
    }
    for path in roles.keys() {
        for other in roles.keys() {
            if other.starts_with(&format!("{path}/")) {
                return Err(format!("file/ancestor conflict: {path} and {other}"));
            }
        }
        parents(&root, path, &mut directories)?;
    }
    let mut files = BTreeMap::new();
    for path in file_inputs
        .into_iter()
        .chain(std::iter::once(manifest.host_context.as_str()))
    {
        let bytes = match pending.get(path) {
            Some(bytes) => bytes.clone(),
            None => required(&root, path)?,
        };
        files.insert(path.to_owned(), bytes);
    }
    if let Some(sources) = selected {
        let conflict = || {
            error(
                &manifest_path,
                "init inputs differ from retained canonical sources or root binding; edit retained sources deliberately and use generate (edit the output plan for locale changes)",
            )
        };
        if sources
            .context
            .is_some_and(|b| b != files[&manifest.host_context])
            || sources.locale.is_some_and(|l| l != manifest.root().locale)
        {
            return Err(conflict());
        }
        if let Some(bytes) = sources.methodology {
            let output = manifest.root();
            let raw = (output.roots.len() == 1)
                .then(|| cat.atoms.iter().find(|a| a.id == output.roots[0]))
                .flatten()
                .filter(|a| a.requires.is_empty())
                .and_then(|a| a.variants.iter().find(|v| v.locale == output.locale))
                .and_then(|v| match &v.source {
                    Content::File { path } => files.get(path),
                    _ => None,
                });
            if raw != Some(&bytes) {
                return Err(conflict());
            }
        }
    }
    for (path, bytes) in pending {
        changes.push(Change::new(root.join(path), None, bytes));
    }
    let mut rendered = Vec::new();
    for output in &manifest.outputs {
        let (body, locale) = cat.compose(output, &files)?;
        if output.format == Format::RootGuide {
            rendered.push((output, managed_block(&manifest, &locale.root_frame, &body)));
        } else {
            rendered.push((output, output.projection(&locale.projection_notice, &body)));
        }
    }
    let block = &rendered
        .iter()
        .find(|(o, _)| o.format == Format::RootGuide)
        .unwrap()
        .1;
    let agents = root.join("AGENTS.md");
    let claude = root.join("CLAUDE.md");
    let alias = metadata(&agents)?.is_some_and(|m| m.file_type().is_symlink());
    let claude_before = regular(&claude)?;
    let agents_before = if alias {
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
        None
    } else {
        regular(&agents)?
    };
    // Sole AGENTS is a donor. For two regular roots, compare prospective bytes,
    // so stale owned blocks do not cause a conflict but different host text does.
    let donor = claude_before.as_ref().or(agents_before.as_ref());
    let donor_path = if claude_before.is_none() && agents_before.is_some() {
        &agents
    } else {
        &claude
    };
    let after = render(
        donor_path,
        donor.map_or(&[], |s| s.bytes.as_slice()),
        &block,
    )?;
    if claude_before.is_some() {
        if let Some(before) = &agents_before {
            if render(&agents, &before.bytes, &block)? != after {
                return Err(format!(
                    "{} and {}: conflicting host-owned content; deliberately consolidate preserved text in CLAUDE.md, then align AGENTS.md or remove the redundant regular AGENTS.md before retrying",
                    agents.display(),
                    claude.display()
                ));
            }
        }
    }
    let permissions = donor.map(|s| s.permissions.clone());
    let mut guide = Change::new(claude, claude_before, after);
    guide.permissions = permissions;
    changes.push(guide);
    if !alias {
        changes.push(Change::alias(agents, agents_before));
    }
    for (output, after) in rendered {
        if output.format == Format::RootGuide {
            continue;
        }
        let path = root.join(&output.path);
        let before = regular(&path)?;
        if let Some(existing) = &before {
            output.check_owned(&existing.bytes)?;
        }
        changes.push(Change::new(path, before, after));
    }
    if new_manifest {
        changes.push(Change::new(
            manifest_path,
            manifest_before,
            catalog::bytes(&manifest)?,
        ));
    }
    changes.retain(|c| !c.unchanged());
    let mut directories: Vec<_> = directories.into_iter().collect();
    directories.sort_by_key(|p| p.components().count());
    Ok((changes, directories))
}

/// Retain exact selected UTF-8 bytes and publish the literal root guide and relative alias.
/// Repeated init refuses differing retained sources. The host must already exist.
pub fn init(host: &Path, methodology: &Path, context: &Path) -> Result<Vec<PathBuf>, String> {
    init_with_defaults(host, Some(methodology), Some(context))
}

/// Initialize with independently optional source files. For a new host, omissions
/// select the embedded general core and empty context. For a registered host,
/// omissions retain validated canonical bytes; explicit conflicts still fail.
pub fn init_with_defaults(
    host: &Path,
    methodology: Option<&Path>,
    context: Option<&Path>,
) -> Result<Vec<PathBuf>, String> {
    let selected = InitSources::read(methodology, context)?;
    let (changes, directories) = prepare(host, Some(selected))?;
    transaction::publish(changes, directories, Fault::default())
}

/// Validate registered sources and refresh the guide and alias while preserving host-owned bytes.
pub fn generate(host: &Path) -> Result<Vec<PathBuf>, String> {
    let (changes, directories) = prepare(host, None)?;
    transaction::publish(changes, directories, Fault::default())
}

/// Bounded IO-failure injection available only in the dedicated test build.
#[cfg(feature = "test-support")]
pub mod test_support {
    use super::*;

    pub fn generate_with_failure(
        host: &Path,
        after: usize,
        rollback: Option<usize>,
    ) -> Result<Vec<PathBuf>, String> {
        let (changes, directories) = prepare(host, None)?;
        transaction::publish(
            changes,
            directories,
            Fault {
                after: Some(after),
                rollback,
            },
        )
    }

    /// Fail before publication `after` (zero based); optionally fail one rollback.
    /// No production CLI or environment switch enables this hook.
    pub fn init_with_failure(
        host: &Path,
        method: &Path,
        context: &Path,
        after: usize,
        rollback: Option<usize>,
    ) -> Result<Vec<PathBuf>, String> {
        let selected = InitSources::read(Some(method), Some(context))?;
        let (changes, directories) = prepare(host, Some(selected))?;
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
        return reply(0, "Usage:\n  chrono-instructions init --host-root H [--methodology M] [--host-context C] [--locale L]\n  chrono-instructions generate --host-root H\nH must be an existing directory. New init adopts embedded atomic rules (zh-CN default) and empty context. --locale binds the fresh root; --methodology retains one opaque file atom (und unless explicitly bound). Registered init retains existing canonical sources for omitted options; explicit differing inputs fail. Optional M/C must be readable UTF-8 regular files. Edit the registered catalog and output plan, then generate root guides, Markdown and skills. AGENTS.md -> CLAUDE.md remains a literal relative alias. Unix only. No host inference or runtime checkout required. Exit 0 complete, 2 usage, 1 generation/IO failure. No judges run.\n".into(), String::new());
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
            "E_USAGE: use --help; --host-root is required, each option may occur only once and needs a value.\n".into(),
        )
    };
    let is_init = args[0] == "init";
    if !is_init && args[0] != "generate" {
        return usage();
    }
    let mut host = None;
    let mut method = None;
    let mut context = None;
    let mut locale = None;
    let mut parts = args[1..].chunks_exact(2);
    for pair in &mut parts {
        let slot = if pair[0] == "--host-root" {
            &mut host
        } else if is_init && pair[0] == "--methodology" {
            &mut method
        } else if is_init && pair[0] == "--host-context" {
            &mut context
        } else if is_init && pair[0] == "--locale" {
            &mut locale
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
        (|| {
            let mut sources = InitSources::read(method.as_deref(), context.as_deref())?;
            sources.locale = locale
                .map(|p| {
                    p.into_os_string()
                        .into_string()
                        .map_err(|_| "locale must be UTF-8".to_string())
                })
                .transpose()?;
            let (changes, directories) = prepare(&host, Some(sources))?;
            transaction::publish(changes, directories, Fault::default())
        })()
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
