use chrono_instructions::{dispatch, generate, init, test_support};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use tempfile::TempDir;

#[path = "../../runner-tests/tests/support/isolated_capability.rs"]
mod isolated_capability;

const METHOD: &str = ".chrono-harness/instructions/methodology.md";
const CONTEXT: &str = ".chrono-harness/instructions/host-context.md";
const MANIFEST: &str = ".chrono-harness/instructions/manifest.json";
const BEGIN: &str = "<!-- chrono-instructions:begin -->";
const END: &str = "<!-- chrono-instructions:end -->";

#[cfg(unix)]
#[test]
fn literal_core_and_relative_alias_are_the_default_layout() {
    let host = Host::new();
    let method = "\u{feff}方法\r\nwithout final newline".as_bytes();
    fs::write(&host.method, method).unwrap();
    host.init().unwrap();
    assert!(
        fs::symlink_metadata(host.root.join("AGENTS.md"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(
        fs::read_link(host.root.join("AGENTS.md"))
            .unwrap()
            .as_os_str(),
        "CLAUDE.md"
    );
    let guide = host.read("CLAUDE.md");
    assert!(guide.windows(method.len()).any(|part| part == method));
    assert_eq!(host.read("AGENTS.md"), guide);
}

fn assert_alias(host: &Host) {
    let path = host.root.join("AGENTS.md");
    assert!(
        fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read_link(path).unwrap().as_os_str(), "CLAUDE.md");
    assert!(
        fs::symlink_metadata(host.root.join("CLAUDE.md"))
            .unwrap()
            .file_type()
            .is_file()
    );
    assert_eq!(host.read("AGENTS.md"), host.read("CLAUDE.md"));
}

fn expected_root(body: &[u8], locale: &str, title: Option<&str>) -> Vec<u8> {
    let asset: serde_json::Value = serde_json::from_slice(
        &fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/instructions/catalog.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let frame = asset["locales"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["id"] == locale)
        .unwrap()["root_frame"]
        .as_str()
        .unwrap();
    let heading = title.map(|t| format!("# {t}\n\n")).unwrap_or_default();
    [BEGIN.as_bytes(), format!("\n{frame}\n\nmanifest: `{MANIFEST}`\ncatalog: `.chrono-harness/instructions/catalog.json`\nhost_context: `{CONTEXT}`\n\n{heading}").as_bytes(), body, b"\n", END.as_bytes(), b"\n"].concat()
}
fn expected_guide(method: &[u8]) -> Vec<u8> {
    expected_root(method, "und", None)
}
fn fresh_default_guide() -> Vec<u8> {
    // Donor/context tests compare with a pristine consumer, independently of
    // presentation order. Layout behavior has its own synthetic contract tests.
    let host = Host::new();
    assert_eq!(cli_init(&host, None, None).exit_code, 0);
    host.read("CLAUDE.md")
}

fn cli_init(
    host: &Host,
    method: Option<&Path>,
    context: Option<&Path>,
) -> chrono_instructions::CliOutput {
    let mut args = vec![
        "init".into(),
        "--host-root".into(),
        host.root.clone().into_os_string(),
    ];
    for (flag, path) in [("--methodology", method), ("--host-context", context)] {
        if let Some(path) = path {
            args.extend([flag.into(), path.as_os_str().to_owned()]);
        }
    }
    dispatch(&args)
}

struct Host {
    temp: TempDir,
    root: PathBuf,
    method: PathBuf,
    context: PathBuf,
}

impl Host {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("host with spaces");
        fs::create_dir(&root).unwrap();
        let method = temp.path().join("selected method.md");
        let context = temp.path().join("selected context.md");
        fs::write(&method, "方法：核对证据。\r\n").unwrap();
        fs::write(&context, b"").unwrap();
        Self {
            temp,
            root,
            method,
            context,
        }
    }

    fn init(&self) -> Result<Vec<PathBuf>, String> {
        init(&self.root, &self.method, &self.context)
    }

    fn write(&self, relative: &str, bytes: impl AsRef<[u8]>) {
        let path = self.root.join(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }

    fn read(&self, relative: &str) -> Vec<u8> {
        fs::read(self.root.join(relative)).unwrap()
    }

    fn snapshot(&self) -> BTreeMap<PathBuf, Vec<u8>> {
        fn walk(root: &Path, dir: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) {
            for entry in fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                // Durable custody is asserted separately. These snapshots keep
                // every business byte/type assertion from the original suite.
                if path == root.join(".chrono-harness/state") {
                    continue;
                }
                let kind = fs::symlink_metadata(&path).unwrap().file_type();
                let value = if kind.is_symlink() {
                    format!("link:{:?}", fs::read_link(&path).unwrap()).into_bytes()
                } else if kind.is_dir() {
                    let before = files.len();
                    walk(root, &path, files);
                    if path == root.join(".chrono-harness") && before == files.len() {
                        continue;
                    }
                    b"directory".to_vec()
                } else {
                    fs::read(&path).unwrap()
                };
                files.insert(path.strip_prefix(root).unwrap().into(), value);
            }
        }
        let mut files = BTreeMap::new();
        walk(&self.root, &self.root, &mut files);
        files
    }
}

#[test]
fn init_retains_exact_inputs_and_publishes_one_literal_guide() {
    let host = Host::new();
    let changed = host.init().unwrap();
    assert_eq!(changed.len(), 6);
    let method = fs::read(&host.method).unwrap();
    assert_eq!(host.read(METHOD), method);
    assert_eq!(host.read(CONTEXT), b"");
    assert_eq!(host.read("CLAUDE.md"), expected_guide(&method));
    assert_alias(&host);
    let manifest: serde_json::Value = serde_json::from_slice(&host.read(MANIFEST)).unwrap();
    assert_eq!(manifest["schema_version"], 2);
    assert_eq!(manifest["producer"], "chrono-instructions");
    assert_eq!(manifest["render"], "atomic-rules/relative-alias/v3");
    assert_eq!(
        manifest["catalog"],
        ".chrono-harness/instructions/catalog.json"
    );
    assert_eq!(manifest["host_context"], CONTEXT);
    assert_eq!(
        manifest["outputs"][0],
        serde_json::json!({"id":"root", "path":"CLAUDE.md", "format":"root-guide", "locale":"und", "roots":["legacy.method"]})
    );
}

#[test]
fn repeated_init_and_generate_do_not_write_and_edits_refresh_only_the_guide() {
    let host = Host::new();
    host.init().unwrap();
    let before = host.snapshot();
    let times: Vec<_> = [METHOD, CONTEXT, MANIFEST, "AGENTS.md", "CLAUDE.md"]
        .iter()
        .map(|p| {
            fs::symlink_metadata(host.root.join(p))
                .unwrap()
                .modified()
                .unwrap()
        })
        .collect();
    assert!(host.init().unwrap().is_empty());
    assert!(generate(&host.root).unwrap().is_empty());
    assert_eq!(host.snapshot(), before);
    for (path, time) in [METHOD, CONTEXT, MANIFEST, "AGENTS.md", "CLAUDE.md"]
        .iter()
        .zip(times)
    {
        assert_eq!(
            fs::symlink_metadata(host.root.join(path))
                .unwrap()
                .modified()
                .unwrap(),
            time
        );
    }
    host.write(METHOD, "自主修订的通用方法\n");
    host.write(CONTEXT, "项目事实：仅使用显式登记。\n");
    let changed = generate(&host.root).unwrap();
    assert_eq!(
        changed,
        vec![fs::canonicalize(&host.root).unwrap().join("CLAUDE.md")]
    );
    assert_eq!(host.read("CLAUDE.md"), expected_guide(&host.read(METHOD)));
    assert_alias(&host);
    let edited = host.snapshot();
    assert!(generate(&host.root).unwrap().is_empty());
    assert!(host.init().unwrap_err().contains("init inputs differ"));
    assert_eq!(host.snapshot(), edited);
    assert!(
        init(
            &host.root,
            &host.root.join(METHOD),
            &host.root.join(CONTEXT)
        )
        .unwrap()
        .is_empty()
    );
    host.write(CONTEXT, "context changes do not rewrite the guide");
    assert!(generate(&host.root).unwrap().is_empty());
}

#[test]
fn preserves_unrelated_bytes_and_only_replaces_owned_span() {
    let host = Host::new();
    let prefix = b"host preface\r\n\xff\x00\n";
    let suffix = b"\r\nhost suffix without trailing newline\xfe";
    let mut original = prefix.to_vec();
    original.extend_from_slice(format!("{BEGIN}\r\nstale route\r\n{END}").as_bytes());
    original.extend_from_slice(suffix);
    host.write("AGENTS.md", &original);
    let mut other = prefix.to_vec();
    other.extend_from_slice(format!("{BEGIN}\nother old block\n{END}").as_bytes());
    other.extend_from_slice(suffix);
    host.write("CLAUDE.md", other);
    host.write(".chrono-harness/keep.json", b"host configuration");
    host.init().unwrap();
    let agents = host.read("AGENTS.md");
    assert!(agents.starts_with(prefix));
    assert!(agents.ends_with(suffix));
    assert!(!String::from_utf8_lossy(&agents).contains("stale route"));
    assert_alias(&host);
    assert_eq!(
        host.read(".chrono-harness/keep.json"),
        b"host configuration"
    );
    let before = host.snapshot();
    assert!(generate(&host.root).unwrap().is_empty());
    assert_eq!(host.snapshot(), before);
}

#[test]
fn refreshes_changed_block_and_recreates_absent_entrypoint() {
    let host = Host::new();
    host.init().unwrap();
    host.write(
        "AGENTS.md",
        format!("local\n{BEGIN}\nmanual change inside owned block\n{END}\ntail"),
    );
    fs::remove_file(host.root.join("AGENTS.md")).unwrap();
    assert_eq!(generate(&host.root).unwrap().len(), 2);
    assert_alias(&host);
    assert!(host.read("AGENTS.md").starts_with(b"local\n"));
    assert!(host.read("AGENTS.md").ends_with(b"\ntail"));
    assert!(host.root.join("CLAUDE.md").is_file());
}

#[test]
fn malformed_markers_fail_before_any_write() {
    let cases = [
        BEGIN.to_owned(),
        END.to_owned(),
        format!("{END}\n{BEGIN}"),
        format!("{BEGIN}\n{BEGIN}\n{END}"),
        format!("{BEGIN}\n{END}\n{BEGIN}\n{END}"),
        format!("inline {BEGIN}\n{END}"),
        format!("{BEGIN} extra\n{END}"),
        format!("{BEGIN}\n{END} extra"),
        "<!-- chrono-instructions:broken -->".into(),
    ];
    for bytes in cases {
        let host = Host::new();
        host.write("AGENTS.md", "safe original");
        host.write("CLAUDE.md", &bytes);
        let before = host.snapshot();
        let error = host.init().unwrap_err();
        assert!(error.contains("managed delimiters"), "{bytes:?}: {error}");
        let result = cli_init(&host, None, None);
        assert_eq!(result.exit_code, 1, "{result:?}");
        assert!(
            result.stderr.contains("managed delimiters"),
            "{bytes:?}: {result:?}"
        );
        assert_eq!(host.snapshot(), before);
    }
}

#[test]
fn invalid_registration_is_rejected_without_repairs_or_output_changes() {
    let host = Host::new();
    host.init().unwrap();
    let original = String::from_utf8(host.read(MANIFEST)).unwrap();
    let cases = [
        "{".to_owned(),
        original.replacen(
            "\"schema_version\": 2",
            "\"schema_version\": 2, \"schema_version\": 2",
            1,
        ),
        original.replacen("\"schema_version\": 2", "\"schema_version\": 99", 1),
        original.replacen(
            "\"schema_version\": 2",
            "\"extra\": true, \"schema_version\": 2",
            1,
        ),
        original.replace("atomic-rules/relative-alias/v3", "atomic-rules/v9"),
        original.replace("\"chrono-instructions\"", "\"different-producer\""),
        original.replace("CLAUDE.md", "AGENTS.md"),
        original.replacen("\"id\": \"root\"", "\"id\": \"root\", \"id\": \"root\"", 1),
        format!("{original} true"),
    ];
    for bytes in cases {
        assert_ne!(bytes, original);
        host.write(MANIFEST, bytes);
        let before = host.snapshot();
        assert!(generate(&host.root).is_err());
        assert_eq!(cli_init(&host, None, None).exit_code, 1);
        assert_eq!(host.snapshot(), before);
    }
}

#[test]
fn init_never_overwrites_reserved_unregistered_material() {
    for path in [METHOD, CONTEXT, ".chrono-harness/instructions/catalog.json"] {
        let host = Host::new();
        host.write(path, "customized existing source");
        let before = host.snapshot();
        assert!(host.init().unwrap_err().contains("collision"));
        let result = cli_init(&host, None, None);
        assert_eq!(result.exit_code, 1, "{result:?}");
        assert!(result.stderr.contains("collision"));
        assert_eq!(host.snapshot(), before);
    }
}

#[test]
fn input_failures_and_absent_registration_are_errors() {
    let host = Host::new();
    assert!(
        generate(&host.root)
            .unwrap_err()
            .contains("missing registration")
    );
    assert!(host.snapshot().is_empty());
    for path in [&host.method, &host.context] {
        let original = fs::read(path).unwrap();
        fs::write(path, [0xff]).unwrap();
        assert!(host.init().unwrap_err().contains("UTF-8"));
        assert!(host.snapshot().is_empty());
        fs::remove_file(path).unwrap();
        assert!(host.init().is_err());
        assert!(host.snapshot().is_empty());
        fs::create_dir(path).unwrap();
        assert!(host.init().unwrap_err().contains("regular input"));
        fs::remove_dir(path).unwrap();
        fs::write(path, original).unwrap();
    }
    let missing = host.temp.path().join("absent host");
    assert!(init(&missing, &host.method, &host.context).is_err());
    assert!(!missing.exists());
    assert!(init(&host.method, &host.method, &host.context).is_err());
}

#[test]
fn generate_reads_both_retained_sources_even_if_guide_is_current() {
    for path in [METHOD, CONTEXT] {
        let host = Host::new();
        host.init().unwrap();
        host.write(path, [0xff]);
        let before = host.snapshot();
        assert!(generate(&host.root).unwrap_err().contains("UTF-8"));
        let result = cli_init(&host, None, None);
        assert_eq!(result.exit_code, 1, "{result:?}");
        assert!(result.stderr.contains("UTF-8"));
        assert_eq!(host.snapshot(), before);
        fs::remove_file(host.root.join(path)).unwrap();
        let before = host.snapshot();
        assert!(
            generate(&host.root)
                .unwrap_err()
                .contains("registered source is missing")
        );
        let result = cli_init(&host, None, None);
        assert_eq!(result.exit_code, 1, "{result:?}");
        assert!(result.stderr.contains("registered source is missing"));
        assert_eq!(host.snapshot(), before);
    }
}

#[test]
fn conflicting_file_types_fail_before_changes() {
    for path in ["AGENTS.md", "CLAUDE.md", MANIFEST, METHOD, CONTEXT] {
        let host = Host::new();
        fs::create_dir_all(host.root.join(path)).unwrap();
        let before = host.snapshot();
        assert!(host.init().unwrap_err().contains("regular file"));
        assert_eq!(cli_init(&host, None, None).exit_code, 1);
        assert_eq!(host.snapshot(), before);
    }
    for path in [".chrono-harness", ".chrono-harness/instructions"] {
        let host = Host::new();
        host.write(path, "file, not directory");
        let before = host.snapshot();
        assert!(host.init().unwrap_err().contains("expected a directory"));
        assert_eq!(cli_init(&host, None, None).exit_code, 1);
        assert_eq!(host.snapshot(), before);
    }
}

#[cfg(unix)]
#[test]
fn recognized_root_alias_is_preserved_and_explicit_input_alias_can_be_selected() {
    use std::ffi::OsStr;
    use std::os::unix::fs::symlink;
    let host = Host::new();
    host.write("CLAUDE.md", "host preface\n");
    symlink("CLAUDE.md", host.root.join("AGENTS.md")).unwrap();
    let selected = host.temp.path().join("selected alias.md");
    symlink(&host.method, &selected).unwrap();
    assert_eq!(init(&host.root, &selected, &host.context).unwrap().len(), 5);
    assert_eq!(
        fs::read_link(host.root.join("AGENTS.md"))
            .unwrap()
            .as_os_str(),
        OsStr::new("CLAUDE.md")
    );
    assert_eq!(host.read("AGENTS.md"), host.read("CLAUDE.md"));
    assert!(host.read("CLAUDE.md").starts_with(b"host preface\n"));
    assert!(generate(&host.root).unwrap().is_empty());
    assert!(host.init().unwrap().is_empty());
    assert!(
        cli_init(&host, None, None)
            .stdout
            .contains("0 file(s) changed")
    );
}

#[cfg(unix)]
#[test]
fn nonliteral_root_aliases_fail_without_changes() {
    use std::os::unix::fs::symlink;
    for target in ["CLAUDE.md/.", "CLAUDE.md/", "./CLAUDE.md"] {
        for registered in [false, true] {
            let host = Host::new();
            if registered {
                host.init().unwrap();
                fs::remove_file(host.root.join("AGENTS.md")).unwrap();
            }
            host.write("CLAUDE.md", "preserve host preface\n");
            symlink(target, host.root.join("AGENTS.md")).unwrap();
            let before = host.snapshot();
            let mut args = vec![
                OsString::from(if registered { "generate" } else { "init" }),
                OsString::from("--host-root"),
                host.root.clone().into_os_string(),
            ];
            if !registered {
                args.extend([
                    OsString::from("--methodology"),
                    host.method.clone().into_os_string(),
                    OsString::from("--host-context"),
                    host.context.clone().into_os_string(),
                ]);
            }
            let result = dispatch(&args);
            assert_eq!(
                result.exit_code, 1,
                "target={target}, registered={registered}"
            );
            assert!(result.stdout.is_empty());
            assert!(result.stderr.contains("only the relative alias"));
            assert_eq!(host.snapshot(), before, "target={target}");
        }
    }
}

#[cfg(unix)]
#[test]
fn unrecognized_or_dangling_aliases_and_linked_storage_are_rejected() {
    use std::os::unix::fs::symlink;
    for (name, target) in [
        ("AGENTS.md", "elsewhere.md"),
        ("CLAUDE.md", "AGENTS.md"),
        ("AGENTS.md", "CLAUDE.md"),
    ] {
        let host = Host::new();
        symlink(target, host.root.join(name)).unwrap();
        let before = host.snapshot();
        assert!(host.init().is_err());
        assert_eq!(host.snapshot(), before);
    }
    for relative in [".chrono-harness", ".chrono-harness/instructions", METHOD] {
        let host = Host::new();
        let destination = host.root.join(relative);
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        symlink(&host.method, destination).unwrap();
        let before = host.snapshot();
        assert!(host.init().is_err());
        assert_eq!(host.snapshot(), before);
    }
    let host = Host::new();
    host.write("CLAUDE.md", "shared inode");
    fs::hard_link(host.root.join("CLAUDE.md"), host.root.join("AGENTS.md")).unwrap();
    let before = host.snapshot();
    assert!(host.init().unwrap_err().contains("alias"));
    assert_eq!(host.snapshot(), before);
}

#[cfg(unix)]
#[test]
fn unreadable_inputs_outputs_and_storage_fail_without_publication() {
    use std::os::unix::fs::PermissionsExt;
    let host = Host::new();
    host.init().unwrap();
    for path in [
        &host.method,
        &host.context,
        &host.root.join(METHOD),
        &host.root.join(CONTEXT),
        &host.root.join("CLAUDE.md"),
        &host.root.join(MANIFEST),
    ] {
        let permissions = fs::metadata(path).unwrap().permissions();
        fs::set_permissions(path, fs::Permissions::from_mode(0)).unwrap();
        // Privileged OS users can bypass mode bits; report that condition honestly.
        if fs::read(path).is_ok() {
            fs::set_permissions(path, permissions).unwrap();
            eprintln!("mode-bit unreadability unavailable for privileged test process");
            continue;
        }
        assert!(host.init().is_err());
        if path.starts_with(&host.root) {
            assert!(generate(&host.root).is_err());
            assert_eq!(cli_init(&host, None, None).exit_code, 1);
        }
        fs::set_permissions(path, permissions).unwrap();
    }
    // Real late staging failure, before publication, with existing root files.
    host.write("AGENTS.md", "preserve");
    let before = host.snapshot();
    let permissions = fs::metadata(&host.root).unwrap().permissions();
    fs::set_permissions(&host.root, fs::Permissions::from_mode(0o555)).unwrap();
    let probe_path = host.root.join("directory-write-probe");
    let directory_writable = match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&probe_path)
    {
        Ok(probe) => {
            drop(probe);
            fs::remove_file(&probe_path).unwrap();
            true
        }
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => false,
        Err(e) => panic!("unexpected directory write probe failure: {e}"),
    };
    let result = generate(&host.root);
    fs::set_permissions(&host.root, permissions).unwrap();
    if directory_writable {
        eprintln!("directory refusal assertion skipped: independent write probe succeeded");
    } else {
        assert!(
            result.is_err(),
            "write probe denied, generation: {result:?}"
        );
        assert_eq!(host.snapshot(), before);
        eprintln!("independent directory write probe denied; generation failed without changes");
    }
}

#[test]
fn late_publication_failure_rolls_back_new_material_and_existing_output_bytes() {
    for after in 0..6 {
        let host = Host::new();
        host.write("AGENTS.md", b"original host\xff");
        host.write("CLAUDE.md", b"original host\xff");
        let before = host.snapshot();
        let failure =
            test_support::init_with_failure(&host.root, &host.method, &host.context, after, None)
                .unwrap_err();
        assert!(failure.contains("injected publication failure"));
        assert!(failure.contains("unrestored=[]"));
        assert_eq!(host.snapshot(), before, "{failure}");
    }
}

#[test]
fn incomplete_rollback_identifies_exact_unrestored_path_and_retains_original() {
    let host = Host::new();
    host.write("AGENTS.md", "original host");
    host.write("CLAUDE.md", "original host");
    let failure =
        test_support::init_with_failure(&host.root, &host.method, &host.context, 5, Some(4))
            .unwrap_err();
    let expected = format!(
        "unrestored=[{:?}]",
        fs::canonicalize(&host.root).unwrap().join("AGENTS.md")
    );
    assert!(failure.contains(&expected), "{failure}");
    assert!(failure.contains("original retained at"));
    assert_eq!(host.read("CLAUDE.md"), b"original host");
    assert!(!host.root.join(".chrono-harness/instructions").exists());
    let mut backups = Vec::new();
    for attempt in fs::read_dir(host.root.join(OP_STORE)).unwrap() {
        for member in fs::read_dir(attempt.unwrap().path()).unwrap() {
            let member = member.unwrap();
            if member
                .file_name()
                .to_string_lossy()
                .starts_with("original-")
            {
                backups.push(member.path());
            }
        }
    }
    assert_eq!(backups.len(), 1);
    assert_eq!(fs::read(&backups[0]).unwrap(), b"original host");
    assert!(failure.contains(&backups[0].display().to_string()));
    assert!(
        fs::symlink_metadata(host.root.join("AGENTS.md"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    fs::rename(&backups[0], host.root.join("AGENTS.md")).unwrap();
    assert!(
        fs::symlink_metadata(host.root.join("AGENTS.md"))
            .unwrap()
            .file_type()
            .is_file()
    );
    assert_eq!(host.read("AGENTS.md"), b"original host");
    host.init().unwrap();
    assert_alias(&host);
}

#[cfg(unix)]
#[test]
fn replacement_and_rollback_preserve_regular_file_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let host = Host::new();
    host.write("AGENTS.md", "original");
    fs::set_permissions(
        host.root.join("AGENTS.md"),
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    test_support::init_with_failure(&host.root, &host.method, &host.context, 5, None).unwrap_err();
    assert_eq!(
        fs::metadata(host.root.join("AGENTS.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
    host.init().unwrap();
    assert_eq!(
        fs::metadata(host.root.join("AGENTS.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
}

#[test]
fn cli_usage_generation_errors_and_success_have_distinct_exit_contracts() {
    let args = |words: &[&str]| words.iter().map(OsString::from).collect::<Vec<_>>();
    for words in [vec![], vec!["help"], vec!["--help"], vec!["--version"]] {
        let output = dispatch(&args(&words));
        assert_eq!(output.exit_code, 0);
        assert!(output.stderr.is_empty());
    }
    for words in [
        vec!["unknown"],
        vec!["generate"],
        vec!["init"],
        vec!["init", "--methodology", "method.md"],
        vec!["init", "--host-root", ".", "--methodology"],
        vec!["init", "--host-root", ".", "--host-context", ""],
        vec!["init", "--host-root", ".", "--unknown", "value"],
        vec![
            "init",
            "--host-root",
            ".",
            "--methodology",
            "a",
            "--methodology",
            "b",
        ],
        vec![
            "init",
            "--host-root",
            ".",
            "--host-context",
            "a",
            "--host-context",
            "b",
        ],
        vec!["generate", "--host-root"],
        vec!["generate", "--host-root", ""],
        vec!["generate", "--host-root", ".", "--host-root", "."],
        vec!["generate", "--host-root", ".", "--methodology", "method.md"],
        vec!["--help", "extra"],
    ] {
        let output = dispatch(&args(&words));
        assert_eq!(output.exit_code, 2, "{words:?}");
        assert!(output.stdout.is_empty());
    }
    let host = Host::new();
    let generate_args = vec![
        "generate".into(),
        "--host-root".into(),
        host.root.clone().into_os_string(),
    ];
    let failure = dispatch(&generate_args);
    assert_eq!(failure.exit_code, 1);
    assert!(failure.stdout.is_empty());
    assert!(failure.stderr.contains("E_GENERATION"));
    let output = dispatch(&[
        "init".into(),
        "--host-context".into(),
        host.context.clone().into_os_string(),
        "--host-root".into(),
        host.root.clone().into_os_string(),
        "--methodology".into(),
        host.method.clone().into_os_string(),
    ]);
    assert_eq!(output.exit_code, 0, "{output:?}");
    assert!(output.stdout.contains("6 file(s) changed"));
    let output = dispatch(&generate_args);
    assert_eq!(output.exit_code, 0);
    assert!(output.stdout.contains("0 file(s) changed"));
}

fn runtime_policy(host: &Host, max_bytes: u64, max_files: usize) {
    host.write(
        ".chrono-harness/logging.json",
        serde_json::to_vec(&serde_json::json!({
            "schema": "chrono-runtime-logging/v1", "level": "info",
            "directory": ".chrono-harness/state/logs/", "max_bytes": max_bytes,
            "max_files": max_files, "stderr": false
        }))
        .unwrap(),
    );
}

fn run_cli(cwd: &Path, args: &[OsString]) -> (u32, Output) {
    let child = Command::new(env!("CARGO_BIN_EXE_chrono-test-instructions"))
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let pid = child.id();
    (pid, child.wait_with_output().unwrap())
}

fn runtime_records(host: &Host) -> Vec<serde_json::Value> {
    String::from_utf8(host.read(".chrono-harness/state/logs/runtime.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[test]
fn cli_runtime_logs_success_at_selected_host_and_preserves_machine_output() {
    let host = Host::new();
    let other = Host::new();
    host.init().unwrap();
    runtime_policy(&host, 8192, 2);
    runtime_policy(&other, 8192, 2);
    let args = vec![
        "generate".into(),
        "--host-root".into(),
        host.root.clone().into_os_string(),
    ];
    let (pid, output) = run_cli(&other.root, &args);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(
        output.stdout,
        b"complete: 0 file(s) changed; no judges executed\n"
    );
    assert!(output.stderr.is_empty(), "{output:?}");
    assert!(!other.root.join(".chrono-harness/state/logs").exists());
    let rows = runtime_records(&host);
    assert_eq!(rows.len(), 2);
    for row in &rows {
        assert_eq!(row["schema"], "chrono-log/v1");
        assert_eq!(row["producer"], "chrono-instructions");
        assert_eq!(row["level"], "info");
        assert_eq!(row["context"]["pid"], pid);
        assert_eq!(row["context"]["operation"], "generate");
        assert_eq!(
            row["context"]["host_root"],
            fs::canonicalize(&host.root).unwrap().to_str().unwrap()
        );
    }
    assert_eq!(rows[0]["event"], "cli.started");
    assert_eq!(rows[1]["event"], "cli.finished");
    assert_eq!(rows[1]["fields"]["exit_code"], 0);

    let (_, version) = run_cli(&host.root, &["--version".into()]);
    assert_eq!(version.status.code(), Some(0));
    assert_eq!(version.stdout, b"chrono-instructions 0.1.0\n");
    assert!(version.stderr.is_empty());
    let rows = runtime_records(&host);
    assert_eq!(rows.len(), 4);
    assert_eq!(rows[2]["context"]["operation"], "--version");
    assert_eq!(rows[3]["event"], "cli.finished");
}

#[test]
fn cli_runtime_logs_original_generation_and_usage_diagnostics() {
    for (operation, exit) in [("generate", 1), ("unknown", 2)] {
        let host = Host::new();
        runtime_policy(&host, 8192, 2);
        let args = vec![
            operation.into(),
            "--host-root".into(),
            host.root.clone().into_os_string(),
        ];
        let expected = dispatch(&args);
        let (_, output) = run_cli(&host.root, &args);
        assert_eq!(output.status.code(), Some(exit), "{output:?}");
        assert_eq!(output.stdout, expected.stdout.as_bytes());
        assert_eq!(output.stderr, expected.stderr.as_bytes());
        let rows = runtime_records(&host);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["event"], "cli.started");
        assert_eq!(rows[1]["event"], "cli.diagnostic");
        assert_eq!(rows[1]["message"], expected.stderr.trim_end());
        assert_eq!(rows[1]["level"], "error");
        assert_eq!(rows[2]["event"], "cli.finished");
        assert_eq!(rows[2]["level"], "error");
        assert_eq!(rows[2]["fields"]["exit_code"], exit);
    }
}

#[test]
fn cli_runtime_reports_initialization_and_write_failures_without_losing_business_result() {
    for invalid_policy in [true, false] {
        let host = Host::new();
        if invalid_policy {
            runtime_policy(&host, 8192, 0);
        } else {
            runtime_policy(&host, 1, 2);
        }
        let (_, output) = run_cli(
            &host.root,
            &[
                "init".into(),
                "--host-root".into(),
                host.root.clone().into_os_string(),
            ],
        );
        assert_eq!(output.status.code(), Some(2), "{output:?}");
        let diagnostic: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(diagnostic["producer"], "chrono-instructions");
        assert_eq!(diagnostic["event"], "runtime.logging_failed");
        if invalid_policy {
            assert!(output.stdout.is_empty());
            assert!(!host.root.join("CLAUDE.md").exists());
            assert_eq!(diagnostic["error"]["error"]["code"], "E_LOG_CONFIG");
        } else {
            assert!(
                String::from_utf8(output.stdout)
                    .unwrap()
                    .starts_with("complete:")
            );
            assert!(host.root.join("CLAUDE.md").is_file());
            assert_eq!(
                diagnostic["error"]["error"]["context"]["operation_exit_code"],
                0
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn cli_accepts_os_path_arguments_without_lossy_conversion() {
    use std::os::unix::ffi::OsStringExt;
    let host = Host::new();
    let root = host
        .temp
        .path()
        .join(OsString::from_vec(b"host-\xff".to_vec()));
    let supported = fs::create_dir(&root).is_ok();
    let output = dispatch(&[
        "init".into(),
        "--host-root".into(),
        root.clone().into_os_string(),
        "--methodology".into(),
        host.method.clone().into_os_string(),
        "--host-context".into(),
        host.context.clone().into_os_string(),
    ]);
    if supported {
        assert_eq!(output.exit_code, 0, "{output:?}");
        assert!(root.join("AGENTS.md").is_file());
    } else {
        assert_eq!(output.exit_code, 1, "{output:?}");
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn default_init_ships_exact_core_empty_context_and_sole_agents_donor() {
    let host = Host::new();
    let asset =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/instructions/catalog.json");
    host.write("AGENTS.md", b"existing agents\r\n");
    host.write(".chrono-harness/unrelated.json", b"keep this");
    fs::remove_file(&host.method).unwrap();
    fs::remove_file(&host.context).unwrap();
    let result = cli_init(&host, None, None);
    assert_eq!(result.exit_code, 0, "{result:?}");
    assert!(result.stdout.contains("5 file(s) changed"));
    assert_eq!(
        host.read(".chrono-harness/instructions/catalog.json"),
        fs::read(&asset).unwrap()
    );
    assert!(!host.root.join(METHOD).exists());
    assert_eq!(host.read(CONTEXT), b"");
    assert_eq!(
        host.read("CLAUDE.md"),
        [b"existing agents\r\n".as_slice(), &fresh_default_guide()].concat()
    );
    assert_alias(&host);
    assert_eq!(host.read(".chrono-harness/unrelated.json"), b"keep this");
}

#[test]
fn init_overrides_are_independent_and_retain_exact_selected_bytes() {
    for (method, context) in [(true, false), (false, true), (true, true)] {
        let host = Host::new();
        fs::write(&host.context, "\u{feff}deliberate host facts\r\n").unwrap();
        let result = cli_init(
            &host,
            method.then_some(&host.method),
            context.then_some(&host.context),
        );
        assert_eq!(result.exit_code, 0, "{result:?}");
        if method {
            assert_eq!(host.read(METHOD), fs::read(&host.method).unwrap());
        } else {
            assert!(!host.root.join(METHOD).exists());
            assert_eq!(host.read("CLAUDE.md"), fresh_default_guide());
        }
        assert_eq!(
            host.read(CONTEXT),
            if context {
                fs::read(&host.context).unwrap()
            } else {
                vec![]
            }
        );
    }
}

#[test]
fn default_reinit_retains_custom_sources_without_writes() {
    for selected in [false, true] {
        let host = Host::new();
        if selected {
            host.init().unwrap();
        } else {
            assert_eq!(cli_init(&host, None, None).exit_code, 0);
        }
        for customized in [false, true] {
            if customized {
                if selected {
                    host.write(METHOD, "customized method after installation\r\n");
                } else {
                    let path = ".chrono-harness/instructions/catalog.json";
                    let mut cat: serde_json::Value =
                        serde_json::from_slice(&host.read(path)).unwrap();
                    cat["atoms"][0]["variants"][0]["source"]["text"] =
                        serde_json::json!("customized goal atom\r\n");
                    host.write(path, serde_json::to_vec(&cat).unwrap());
                }
                host.write(CONTEXT, "customized context after installation");
                let manifest: serde_json::Value =
                    serde_json::from_slice(&host.read(MANIFEST)).unwrap();
                host.write(MANIFEST, serde_json::to_vec(&manifest).unwrap());
                assert_eq!(generate(&host.root).unwrap().len(), 1);
            }
            let before = host.snapshot();
            let times: Vec<_> = before
                .keys()
                .map(|p| {
                    fs::symlink_metadata(host.root.join(p))
                        .unwrap()
                        .modified()
                        .unwrap()
                })
                .collect();
            let result = cli_init(&host, None, None);
            assert_eq!(result.exit_code, 0, "{result:?}");
            assert!(result.stdout.contains("0 file(s) changed"));
            assert_eq!(host.snapshot(), before);
            for (path, time) in before.keys().zip(times) {
                assert_eq!(
                    fs::symlink_metadata(host.root.join(path))
                        .unwrap()
                        .modified()
                        .unwrap(),
                    time
                );
            }
        }
    }
}

#[test]
fn registered_init_compares_only_explicit_overrides_and_never_overwrites_conflicts() {
    let host = Host::new();
    host.init().unwrap();
    host.write(METHOD, "custom method\r\n");
    host.write(CONTEXT, "custom context\r\n");
    assert_eq!(generate(&host.root).unwrap().len(), 1);
    let before = host.snapshot();
    for path in [METHOD, CONTEXT] {
        let retained = host.root.join(path);
        let method = (path == METHOD).then_some(retained.as_path());
        let context = (path == CONTEXT).then_some(retained.as_path());
        let result = cli_init(&host, method, context);
        assert_eq!(result.exit_code, 0, "{result:?}");
        assert!(result.stdout.contains("0 file(s) changed"));
    }
    for (method, context) in [
        (Some(host.method.as_path()), None),
        (None, Some(host.context.as_path())),
        (Some(host.method.as_path()), Some(host.context.as_path())),
    ] {
        let result = cli_init(&host, method, context);
        assert_eq!(result.exit_code, 1, "{result:?}");
        assert!(result.stderr.contains("init inputs differ"));
        assert!(
            result
                .stderr
                .contains("edit retained sources deliberately and use generate")
        );
        assert_eq!(host.snapshot(), before);
    }
}

#[test]
fn each_optional_external_input_still_requires_readable_regular_utf8_bytes() {
    for registered in [false, true] {
        for method in [false, true] {
            let host = Host::new();
            if registered {
                host.init().unwrap();
            }
            let before = host.snapshot();
            let path = if method { &host.method } else { &host.context };
            fs::remove_file(path).unwrap();
            for kind in ["missing", "directory", "invalid-utf8"] {
                match kind {
                    "directory" => fs::create_dir(path).unwrap(),
                    "invalid-utf8" => {
                        fs::remove_dir(path).unwrap();
                        fs::write(path, [0xff]).unwrap();
                    }
                    _ => {}
                }
                let result = cli_init(&host, method.then_some(path), (!method).then_some(path));
                assert_eq!(result.exit_code, 1, "{kind}: {result:?}");
                assert!(result.stdout.is_empty());
                assert_eq!(host.snapshot(), before);
            }
        }
    }
}

#[test]
fn exact_method_bytes_survive_framing_edits_and_init_refresh() {
    for method in [
        b"".as_slice(),
        b"plain",
        b"lines\r\n\r\n",
        "\u{feff}首行\r\n末行".as_bytes(),
    ] {
        let host = Host::new();
        fs::write(&host.method, method).unwrap();
        host.init().unwrap();
        assert_eq!(host.read(METHOD), method);
        assert_eq!(host.read("CLAUDE.md"), expected_guide(method));
        assert_alias(&host);
        let edited = [method, b"\nnext version without final newline"].concat();
        host.write(METHOD, &edited);
        let result = cli_init(&host, None, None);
        assert_eq!(result.exit_code, 0, "{result:?}");
        assert!(result.stdout.contains("1 file(s) changed"));
        assert_eq!(host.read("CLAUDE.md"), expected_guide(&edited));
        assert!(generate(&host.root).unwrap().is_empty());
    }
}

#[test]
fn reserved_method_prefix_is_rejected_before_publication() {
    for method in [
        BEGIN.to_owned(),
        END.to_owned(),
        "inline <!-- chrono-instructions-invalid".into(),
    ] {
        for registered in [false, true] {
            let host = Host::new();
            if registered {
                host.init().unwrap();
                host.write(METHOD, &method);
            }
            fs::write(&host.method, &method).unwrap();
            let before = host.snapshot();
            assert!(host.init().unwrap_err().contains("reserved marker prefix"));
            if registered {
                assert!(
                    generate(&host.root)
                        .unwrap_err()
                        .contains("reserved marker prefix")
                );
                assert_eq!(cli_init(&host, None, None).exit_code, 1);
            }
            assert_eq!(host.snapshot(), before);
        }
    }
    // Context is not embedded, so the restriction does not apply there.
    let host = Host::new();
    fs::write(&host.context, BEGIN).unwrap();
    host.init().unwrap();
    assert_eq!(host.read(CONTEXT), BEGIN.as_bytes());
    assert!(generate(&host.root).unwrap().is_empty());
}

#[test]
fn roots_convert_only_when_prospective_whole_bytes_agree() {
    for roots in [
        vec![],
        vec!["AGENTS.md"],
        vec!["CLAUDE.md"],
        vec!["AGENTS.md", "CLAUDE.md"],
    ] {
        let host = Host::new();
        for name in &roots {
            host.write(name, b"host text without newline\xff");
        }
        host.init().unwrap();
        assert_alias(&host);
        let expected = if roots.is_empty() {
            expected_guide(&host.read(METHOD))
        } else {
            [
                b"host text without newline\xff\n".as_slice(),
                &expected_guide(&host.read(METHOD)),
            ]
            .concat()
        };
        assert_eq!(host.read("CLAUDE.md"), expected);
        assert!(generate(&host.root).unwrap().is_empty());
    }
    for registered in [false, true] {
        let host = Host::new();
        if registered {
            host.init().unwrap();
            fs::remove_file(host.root.join("AGENTS.md")).unwrap();
        }
        host.write("AGENTS.md", b"agent-specific text");
        host.write("CLAUDE.md", b"claude-specific text");
        let before = host.snapshot();
        let failure = if registered {
            generate(&host.root)
        } else {
            host.init()
        }
        .unwrap_err();
        assert!(failure.contains("AGENTS.md") && failure.contains("CLAUDE.md"));
        assert!(failure.contains("consolidate"));
        assert_eq!(host.snapshot(), before);
        assert_eq!(cli_init(&host, None, None).exit_code, 1);
        assert_eq!(host.snapshot(), before);
        // Explicit consolidation preserves both original texts; retry can finish.
        host.write("CLAUDE.md", b"agent-specific text\nclaude-specific text");
        fs::remove_file(host.root.join("AGENTS.md")).unwrap();
        assert_eq!(cli_init(&host, None, None).exit_code, 0);
        assert_alias(&host);
        assert!(
            host.read("CLAUDE.md")
                .starts_with(b"agent-specific text\nclaude-specific text\n")
        );
    }
}

#[test]
fn unsupported_manifest_preserves_unmanaged_sources_and_roots() {
    for init_command in [false, true] {
        let host = Host::new();
        host.write(METHOD, fs::read(&host.method).unwrap());
        host.write(CONTEXT, b"deliberate nonempty context\r\n");
        host.write(
            MANIFEST,
            serde_json::to_vec(&serde_json::json!({
                "schema_version": 1,
                "producer": "chrono-instructions/0.1.0",
                "render": "read-both/v1",
                "sources": [
                    {"role": "methodology", "path": METHOD},
                    {"role": "host-context", "path": CONTEXT}
                ],
                "outputs": [
                    {"role": "agents-entrypoint", "path": "AGENTS.md"},
                    {"role": "claude-entrypoint", "path": "CLAUDE.md"}
                ]
            }))
            .unwrap(),
        );
        for name in ["AGENTS.md", "CLAUDE.md"] {
            host.write(
                name,
                format!("host preface\n{BEGIN}\nold reading route\n{END}\nhost tail"),
            );
        }
        let before = host.snapshot();
        let result = dispatch(&[
            if init_command {
                "init".into()
            } else {
                "generate".into()
            },
            "--host-root".into(),
            host.root.clone().into_os_string(),
        ]);
        assert_eq!(result.exit_code, 1, "{result:?}");
        assert!(result.stderr.contains(MANIFEST), "{result:?}");
        assert_eq!(host.snapshot(), before);
    }
}

#[cfg(unix)]
#[test]
fn existing_alias_inode_survives_edits_and_repeated_init() {
    use std::os::unix::fs::MetadataExt;
    let host = Host::new();
    host.init().unwrap();
    let identity = || {
        let m = fs::symlink_metadata(host.root.join("AGENTS.md")).unwrap();
        (m.dev(), m.ino(), m.modified().unwrap())
    };
    let before = identity();
    assert!(generate(&host.root).unwrap().is_empty());
    host.write(METHOD, "changed method");
    assert_eq!(generate(&host.root).unwrap().len(), 1);
    assert_eq!(cli_init(&host, None, None).exit_code, 0);
    assert_eq!(identity(), before);
    assert_alias(&host);
}

#[test]
fn failure_after_fresh_alias_publication_removes_all_new_material() {
    let host = Host::new();
    let failure = test_support::init_with_failure(&host.root, &host.method, &host.context, 5, None)
        .unwrap_err();
    assert!(failure.contains("injected publication failure"));
    assert!(failure.contains("AGENTS.md") && failure.contains("unrestored=[]"));
    assert!(host.snapshot().is_empty(), "{failure}");
    host.init().unwrap();
    assert_alias(&host);
}

#[test]
fn incomplete_fresh_alias_rollback_names_unrestored_link_and_allows_explicit_recovery() {
    if !isolated_capability::isolated_capability_test(
        "incomplete_fresh_alias_rollback_names_unrestored_link_and_allows_explicit_recovery",
    ) {
        return;
    }
    let host = Host::new();
    let failure =
        test_support::init_with_failure(&host.root, &host.method, &host.context, 5, Some(4))
            .unwrap_err();
    let path = fs::canonicalize(&host.root).unwrap().join("AGENTS.md");
    assert!(
        failure.contains(&format!("unrestored=[{path:?}]")),
        "{failure}"
    );
    assert!(
        fs::symlink_metadata(&path)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(!host.root.join("CLAUDE.md").exists());
    // The next call repairs this same attempt before it prepares a new init.
    chrono_instructions::maintain(&host.root).unwrap();
    assert!(!path.exists());
    assert!(host.snapshot().is_empty());
    host.init().unwrap();
    assert_alias(&host);
}

#[path = "atomic.rs"]
mod atomic;

#[test]
fn standalone_noop_has_durable_operation_state() {
    let host = Host::new();
    host.init().unwrap();
    let guide = host.root.join("CLAUDE.md");
    let before = fs::metadata(&guide).unwrap().modified().unwrap();
    assert!(generate(&host.root).unwrap().is_empty());
    assert_eq!(fs::metadata(&guide).unwrap().modified().unwrap(), before);
    let state = host
        .root
        .join(".chrono-harness/state/instructions-operations");
    assert!(state.read_dir().unwrap().count() >= 2);
}

const OP_STORE: &str = ".chrono-harness/state/instructions-operations";
const OP_PRODUCER: &str = "instructions-operation";

fn attempts(host: &Host) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(host.root.join(OP_STORE))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    paths.sort();
    paths
}

fn operation_json(path: &Path, name: &str) -> serde_json::Value {
    serde_json::from_slice(&fs::read(path.join(name)).unwrap()).unwrap()
}

#[test]
fn all_publication_entries_record_success_failure_and_noop_without_rewriting_targets() {
    let host = Host::new();
    host.init().unwrap();
    let guide = host.root.join("CLAUDE.md");
    let old = fs::metadata(&guide).unwrap();
    assert!(
        chrono_instructions::init_with_defaults(&host.root, None, None)
            .unwrap()
            .is_empty()
    );
    assert!(generate(&host.root).unwrap().is_empty());
    let cli = dispatch(&[
        "generate".into(),
        "--host-root".into(),
        host.root.clone().into_os_string(),
    ]);
    assert_eq!(cli.exit_code, 0);
    let now = fs::metadata(&guide).unwrap();
    use std::os::unix::fs::MetadataExt;
    assert_eq!(
        (old.ino(), old.mode(), old.modified().unwrap()),
        (now.ino(), now.mode(), now.modified().unwrap())
    );
    let paths = attempts(&host);
    assert_eq!(paths.len(), 4);
    let mut noops = 0;
    for path in paths {
        let result = operation_json(&path, "result.json");
        assert_eq!(result["publication"], "committed");
        assert!(result["original_process_exit"].is_null());
        assert!(result["transport_exit"].is_null());
        if result["producer_evaluation"]["changed"]
            .as_array()
            .unwrap()
            .is_empty()
        {
            noops += 1;
        }
    }
    assert_eq!(noops, 3);
    host.write(METHOD, [0xff]);
    assert!(generate(&host.root).is_err());
    let failed = attempts(&host)
        .into_iter()
        .map(|p| operation_json(&p, "result.json"))
        .find(|v| v["producer_evaluation"].get("error").is_some())
        .unwrap();
    assert_eq!(failed["publication"], "not-started");
}

#[test]
fn publication_finishes_sealing_before_reporting_init_generate_and_noop_success() {
    let host = Host::new();
    for init in [true, false, false] {
        if !init {
            host.write(METHOD, "body after init");
        }
        let changed = test_support::operation_with_sealing_round_budget(&host.root, init, 1)
            .unwrap_or_else(|error| panic!("init={init}: {error}"));
        if init {
            assert!(!changed.is_empty());
        }
        let state = chrono_instructions::status(&host.root, None).unwrap();
        for record in state["records"].as_array().unwrap() {
            assert_eq!(record["phase"], "released", "{record}");
            assert_eq!(record["sealed"], true, "{record}");
        }
        let inventory =
            chrono_harness::retained_artifacts::retention::Inventory::with_standalone_policy(
                &host.root,
                include_bytes!("../../../assets/instructions/standalone-retention.json"),
            )
            .unwrap();
        assert!(
            inventory
                .rooted_original(
                    "instructions-operation",
                    "instructions-publication-recovery"
                )
                .unwrap()
                .is_none()
        );
    }
    assert!(generate(&host.root).unwrap().is_empty());
}

#[test]
fn finalization_failure_is_real_and_the_same_attempt_is_recoverable() {
    // Recovery observes immediate last-close metadata ownership. Run this
    // host without unrelated libtest forks inheriting its open capabilities.
    if !isolated_capability::isolated_capability_test(
        "finalization_failure_is_real_and_the_same_attempt_is_recoverable",
    ) {
        return;
    }
    for point in [
        "cleanup",
        "before-result",
        "before-seal",
        "sealed",
        "release",
    ] {
        let host = Host::new();
        host.init().unwrap();
        host.write(METHOD, "new body");
        let error = test_support::generate_with_state_failure(&host.root, point).unwrap_err();
        assert!(error.contains(point), "{error}");
        let count = attempts(&host).len();
        chrono_instructions::maintain(&host.root).unwrap();
        assert_eq!(
            attempts(&host).len(),
            count,
            "recovery must not dispatch a replacement"
        );
        assert!(
            String::from_utf8(host.read("CLAUDE.md"))
                .unwrap()
                .contains("new body")
        );
        assert!(generate(&host.root).unwrap().is_empty());
    }
}

// All subprocesses below are this Rust test unit. No external tool is executed.
#[test]
#[ignore]
fn recovery_child() {
    use std::io::{Read, Write};
    fn boundary(point: &str) {
        if std::env::var("CHRONO_INSTRUCTIONS_BOUNDARY").as_deref() == Ok(point) {
            println!("BOUNDARY {point}");
            std::io::stdout().flush().unwrap();
            let mut byte = [0];
            std::io::stdin().read_exact(&mut byte).unwrap();
        }
    }
    let root = PathBuf::from(std::env::var_os("CHRONO_INSTRUCTIONS_HOST").unwrap());
    let mode = std::env::var("CHRONO_INSTRUCTIONS_MODE").unwrap();
    if mode == "raw-init" {
        let method = PathBuf::from(std::env::var_os("CHRONO_INSTRUCTIONS_METHOD").unwrap());
        let context = PathBuf::from(std::env::var_os("CHRONO_INSTRUCTIONS_CONTEXT").unwrap());
        test_support::init_with_checkpoint(&root, &method, &context, boundary).unwrap();
    } else {
        test_support::operation_with_checkpoint(&root, mode == "init", boundary).unwrap();
    }
}

struct PausedOperation {
    child: std::process::Child,
    output: std::io::BufReader<std::process::ChildStdout>,
}
impl PausedOperation {
    fn start(host: &Host, point: &str, mode: &str) -> Self {
        use std::io::BufRead;
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "recovery_child",
                "--nocapture",
                "--test-threads=1",
            ])
            .env("CHRONO_INSTRUCTIONS_HOST", &host.root)
            .env("CHRONO_INSTRUCTIONS_BOUNDARY", point)
            .env("CHRONO_INSTRUCTIONS_MODE", mode)
            .env("CHRONO_INSTRUCTIONS_METHOD", &host.method)
            .env("CHRONO_INSTRUCTIONS_CONTEXT", &host.context)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let output = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut paused = Self { child, output };
        let mut line = String::new();
        loop {
            line.clear();
            if paused.output.read_line(&mut line).unwrap() == 0 {
                use std::io::Read;
                let status = paused.child.wait().unwrap();
                let mut stderr = String::new();
                paused
                    .child
                    .stderr
                    .take()
                    .unwrap()
                    .read_to_string(&mut stderr)
                    .unwrap();
                panic!("child ended before {point}: {status}; {stderr}");
            }
            if line.contains(&format!("BOUNDARY {point}")) {
                break;
            }
        }
        paused
    }
    fn interrupt(mut self) {
        self.child.kill().unwrap();
        let status = self.child.wait().unwrap();
        use std::os::unix::process::ExitStatusExt;
        assert!(status.signal().is_some());
    }
    fn finish_with_failure(mut self) -> String {
        use std::io::{Read, Write};
        self.child.stdin.take().unwrap().write_all(b"x").unwrap();
        let mut stdout = String::new();
        self.output.read_to_string(&mut stdout).unwrap();
        let status = self.child.wait().unwrap();
        let mut stderr = String::new();
        self.child
            .stderr
            .take()
            .unwrap()
            .read_to_string(&mut stderr)
            .unwrap();
        assert!(!status.success(), "{stdout}; {stderr}");
        stderr
    }
    fn finish(mut self) {
        use std::io::{Read, Write};
        self.child.stdin.take().unwrap().write_all(b"x").unwrap();
        let mut output = String::new();
        self.output.read_to_string(&mut output).unwrap();
        assert!(self.child.wait().unwrap().success(), "{output}");
    }
}
impl Drop for PausedOperation {
    fn drop(&mut self) {
        // Test assertion failures still join their own Rust child.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn independent_process_interruptions_recover_before_prepare_and_preserve_unknown_exit() {
    if !isolated_capability::isolated_capability_test(
        "independent_process_interruptions_recover_before_prepare_and_preserve_unknown_exit",
    ) {
        return;
    }
    for point in [
        "intent",
        "before-prepare",
        "staged-dir-0",
        "staged-file-0",
        "prepared",
        "published-dir-0",
        "published-0",
        "published-4",
        "committed",
        "before-result",
        "sealed",
    ] {
        let host = Host::new();
        PausedOperation::start(&host, point, "init").interrupt();
        chrono_instructions::maintain(&host.root)
            .unwrap_or_else(|error| panic!("{point}: {error}"));
        assert_eq!(attempts(&host).len(), 1, "{point}");
        let path = attempts(&host).pop().unwrap();
        let result = operation_json(&path, "result.json");
        assert!(result["original_process_exit"].is_null());
        assert!(
            matches!(
                result["publication"].as_str(),
                Some("not-started" | "restored" | "verified-committed" | "committed")
            ),
            "{result}"
        );
        let state = chrono_instructions::status(
            &host.root,
            Some(path.strip_prefix(&host.root).unwrap().to_str().unwrap()),
        )
        .unwrap();
        assert_eq!(state["records"][0]["phase"], "released");
        assert_eq!(
            dispatch(&[
                "init".into(),
                "--host-root".into(),
                host.root.clone().into_os_string()
            ])
            .exit_code,
            0
        );
        assert_alias(&host);
    }
}

#[test]
fn recovery_itself_can_be_interrupted_and_resumes_the_original_backup() {
    use std::os::unix::fs::PermissionsExt;
    let host = Host::new();
    host.init().unwrap();
    fs::set_permissions(
        host.root.join("CLAUDE.md"),
        fs::Permissions::from_mode(0o640),
    )
    .unwrap();
    let original = host.read("CLAUDE.md");
    host.write(METHOD, "replacement");
    PausedOperation::start(&host, "published-0", "generate").interrupt();
    PausedOperation::start(&host, "restored-file-0", "generate").interrupt();
    chrono_instructions::maintain(&host.root).unwrap();
    assert_eq!(attempts(&host).len(), 2);
    assert_eq!(host.read("CLAUDE.md"), original);
    assert_eq!(
        fs::metadata(host.root.join("CLAUDE.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o640
    );
    assert_alias(&host);
}

#[test]
fn same_host_excludes_a_second_process_while_an_independent_host_progresses() {
    let first = Host::new();
    let second = Host::new();
    let paused = PausedOperation::start(&first, "before-prepare", "init");
    let error = chrono_instructions::init_with_defaults(&first.root, None, None).unwrap_err();
    assert!(error.contains("busy"), "{error}");
    second.init().unwrap();
    assert_alias(&second);
    paused.finish();
    assert_alias(&first);
}

#[test]
fn completed_synchronous_operation_releases_lock_while_unrelated_fork_retains_descriptors() {
    use std::io::Read;
    use std::os::fd::AsRawFd;
    use std::os::unix::net::UnixStream;
    let host = Host::new();
    host.init().unwrap();
    let lease = chrono_harness::ownership::DirectoryLease::acquire_local(
        &host
            .root
            .join(".chrono-harness/state/instructions-operations"),
        true,
        None,
    )
    .unwrap()
    .unwrap();
    let (read, write) = UnixStream::pair().unwrap();
    let child = unsafe { libc::fork() };
    assert!(child >= 0);
    if child == 0 {
        // This unrelated pre-exec fork retains the open descriptions. It only
        // uses async-signal-safe syscalls, and EOF is its native completion gate.
        unsafe {
            libc::close(write.as_raw_fd());
            if libc::write(read.as_raw_fd(), b"r".as_ptr().cast(), 1) != 1 {
                libc::_exit(90);
            }
            let mut byte = 0u8;
            loop {
                let result = libc::read(read.as_raw_fd(), (&mut byte as *mut u8).cast(), 1);
                if result == 0 {
                    break;
                }
                #[cfg(target_os = "macos")]
                let interrupted = *libc::__error() == libc::EINTR;
                #[cfg(target_os = "linux")]
                let interrupted = *libc::__errno_location() == libc::EINTR;
                if result < 0 && interrupted {
                    continue;
                }
                libc::_exit(91);
            }
            libc::_exit(0);
        }
    }
    struct JoinedFork {
        write: Option<UnixStream>,
        child: libc::pid_t,
    }
    impl Drop for JoinedFork {
        fn drop(&mut self) {
            drop(self.write.take());
            let mut status = 0;
            assert_eq!(
                unsafe { libc::waitpid(self.child, &mut status, 0) },
                self.child
            );
            assert_eq!(status, 0);
        }
    }
    let mut joined = JoinedFork {
        write: Some(write),
        child,
    };
    let mut ready = [0];
    joined
        .write
        .as_mut()
        .unwrap()
        .read_exact(&mut ready)
        .unwrap();
    assert_eq!(ready, *b"r");
    let busy = cli_init(&host, None, None);
    assert_eq!(busy.exit_code, 1, "{busy:?}");
    assert!(busy.stderr.contains("busy"));
    drop(lease);
    // The inherited descriptor remains live. The real next caller must succeed
    // immediately without a retry, sleep or weakened same-host exclusion.
    let result = cli_init(&host, None, None);
    assert_eq!(result.exit_code, 0, "{result:?}");
    assert_alias(&host);
}

#[test]
fn recovery_preserves_destination_parent_input_and_literal_alias_drift() {
    for drift in ["destination", "parent", "input", "alias"] {
        let host = Host::new();
        host.init().unwrap();
        host.write(METHOD, "next");
        PausedOperation::start(&host, "published-0", "generate").interrupt();
        match drift {
            "destination" => {
                fs::remove_file(host.root.join("CLAUDE.md")).unwrap();
                host.write("CLAUDE.md", "independent work");
            }
            "parent" => {
                fs::rename(
                    host.root.join(".chrono-harness/instructions"),
                    host.root.join(".chrono-harness/previous-instructions"),
                )
                .unwrap();
                fs::create_dir(host.root.join(".chrono-harness/instructions")).unwrap();
            }
            "input" => host.write(METHOD, "independent input"),
            "alias" => {
                fs::remove_file(host.root.join("AGENTS.md")).unwrap();
                std::os::unix::fs::symlink("other.md", host.root.join("AGENTS.md")).unwrap();
            }
            _ => unreachable!(),
        }
        let before = host.snapshot();
        let error = chrono_instructions::maintain(&host.root).unwrap_err();
        assert!(!error.is_empty());
        assert_eq!(host.snapshot(), before);
        assert_eq!(attempts(&host).len(), 2);
        let inventory =
            chrono_harness::retained_artifacts::retention::Inventory::with_standalone_policy(
                &host.root,
                include_bytes!("../../../assets/instructions/standalone-retention.json"),
            )
            .unwrap();
        assert!(
            inventory
                .rooted_original(OP_PRODUCER, "instructions-publication-recovery")
                .unwrap()
                .is_some()
        );
    }
}

#[test]
fn commit_and_result_must_bind_the_original_intent_and_plan() {
    for record in ["plan.json", "commit.json", "result.json"] {
        let host = Host::new();
        host.init().unwrap();
        host.write(METHOD, "next");
        let point = if record == "result.json" {
            "sealed"
        } else {
            "committed"
        };
        PausedOperation::start(&host, point, "generate").interrupt();
        let pending = attempts(&host)
            .into_iter()
            .find(|p| {
                operation_json(p, "plan.json")["files"]
                    .as_array()
                    .is_some_and(|a| a.len() == 1)
            })
            .unwrap();
        let mut value = operation_json(&pending, record);
        if record == "plan.json" {
            value["host"] = serde_json::json!("different-host");
        } else {
            value["attempt"] = serde_json::json!("different-attempt");
        }
        fs::write(pending.join(record), serde_json::to_vec(&value).unwrap()).unwrap();
        let before = host.snapshot();
        assert!(chrono_instructions::maintain(&host.root).is_err());
        assert_eq!(host.snapshot(), before);
    }
}

fn retention_policy(host: &Host, count: u64, bytes: u64) {
    let mut policy: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../assets/instructions/standalone-retention.json"
    ))
    .unwrap();
    policy["terminal_limits"][OP_PRODUCER]["max_count"] = count.into();
    policy["terminal_limits"][OP_PRODUCER]["max_bytes"] = bytes.into();
    host.write(
        ".chrono-harness/retention.json",
        serde_json::to_vec(&policy).unwrap(),
    );
}

fn inventory(host: &Host) -> chrono_harness::retained_artifacts::retention::Inventory {
    chrono_harness::retained_artifacts::retention::Inventory::with_standalone_policy(
        &host.root,
        include_bytes!("../../../assets/instructions/standalone-retention.json"),
    )
    .unwrap()
}

fn maintain_until(
    state: &chrono_harness::retained_artifacts::retention::Inventory,
    observed: Option<u64>,
    complete: impl Fn() -> bool,
) -> Vec<serde_json::Value> {
    // Maintenance is an explicit bounded action, not a synchronous drain.
    // Drive a finite sequence of real rounds without changing the policy's
    // time/node/byte bounds, and retain every round for failure diagnostics.
    let mut rounds = Vec::new();
    for _ in 0..32 {
        let result = match observed {
            Some(now) => state.maintain_producer_at(OP_PRODUCER, now),
            None => state.maintain_producer(OP_PRODUCER),
        }
        .unwrap();
        assert!(
            result["quota_errors"].as_array().unwrap().is_empty(),
            "{result}"
        );
        assert!(
            result["round"]["accounting_errors"]
                .as_array()
                .unwrap()
                .is_empty(),
            "{result}"
        );
        assert!(
            result["round"]["nodes"].as_u64().unwrap() <= 256,
            "{result}"
        );
        rounds.push(result);
        if complete() {
            assert!(
                rounds
                    .iter()
                    .any(|r| r["round"]["removed_logical_bytes"].as_u64().unwrap() > 0),
                "no observed physical progress: {rounds:?}"
            );
            return rounds;
        }
    }
    panic!("bounded maintenance did not retire the expected originals: {rounds:?}");
}

#[test]
fn terminal_count_and_bytes_are_maintenance_targets_with_truthful_retired_queries() {
    // Immediate metadata lock acquisition must not depend on unrelated fixture
    // forks briefly inheriting this host's open capabilities before exec.
    if !isolated_capability::isolated_capability_test(
        "terminal_count_and_bytes_are_maintenance_targets_with_truthful_retired_queries",
    ) {
        return;
    }
    for (count, bytes, expected) in [(1, 67108864, 1), (32, 1, 0)] {
        let host = Host::new();
        retention_policy(&host, count, bytes);
        host.init().unwrap();
        generate(&host.root).unwrap();
        let paths = attempts(&host);
        let registered = inventory(&host).producer_status(OP_PRODUCER, None).unwrap();
        let retired_ids: Vec<_> = registered["records"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                (
                    r["id"].as_str().unwrap().to_owned(),
                    host.root.join(r["path"].as_str().unwrap()),
                )
            })
            .collect();
        let state = inventory(&host);
        let rounds = maintain_until(&state, None, || {
            let status = state.producer_status(OP_PRODUCER, None).unwrap();
            assert!(status["errors"].as_array().unwrap().is_empty(), "{status}");
            status["scope_complete"] == true
                && status["records"].as_array().unwrap().len() == expected
                && paths.iter().filter(|p| p.exists()).count() == expected
                && retired_ids.iter().all(|(id, path)| {
                    path.exists()
                        || !host
                            .root
                            .join(".chrono-harness/state/output-retention-v1/objects")
                            .join(id)
                            .exists()
                })
        });
        assert!(rounds.iter().any(|r| r["quota_scope_complete"] == true));
        assert!(
            rounds
                .iter()
                .any(|r| !r["quota_expired"].as_array().unwrap().is_empty())
        );
        let status = chrono_instructions::status(&host.root, None).unwrap();
        assert_eq!(status["scope_complete"], true, "{status}");
        assert_eq!(
            status["records"].as_array().unwrap().len(),
            expected,
            "{status}"
        );
        for path in paths.into_iter().filter(|p| !p.exists()) {
            let retired = chrono_instructions::status(
                &host.root,
                Some(path.strip_prefix(&host.root).unwrap().to_str().unwrap()),
            )
            .unwrap();
            assert_eq!(retired["availability"], "unavailable-or-retired");
            assert_eq!(retired["success_inferred"], false);
        }
        for (id, path) in retired_ids.into_iter().filter(|(_, p)| !p.exists()) {
            assert!(
                !host
                    .root
                    .join(".chrono-harness/state/output-retention-v1/objects")
                    .join(id)
                    .exists()
            );
            assert!(!path.exists());
        }
        assert_alias(&host);
    }
}

#[test]
fn age_and_quota_maintenance_preserve_live_consumers_and_unresolved_attempts() {
    let host = Host::new();
    retention_policy(&host, 1, 1);
    host.init().unwrap();
    let state = inventory(&host);
    let status = state.producer_status(OP_PRODUCER, None).unwrap();
    let id = status["records"][0]["id"].as_str().unwrap();
    let original = host
        .root
        .join(status["records"][0]["path"].as_str().unwrap());
    state.reference("test-reader", &[id.into()]).unwrap();
    let live = state.consume(id).unwrap();
    let result = state
        .maintain_producer_at(OP_PRODUCER, u64::MAX - 1)
        .unwrap();
    assert!(original.exists(), "{result}");
    state.reference("test-reader", &[]).unwrap();
    let result = state
        .maintain_producer_at(OP_PRODUCER, u64::MAX - 1)
        .unwrap();
    assert!(original.exists(), "live read capability: {result}");
    drop(live);
    maintain_until(&state, Some(u64::MAX - 1), || !original.exists());
    assert!(!original.exists());
    let paused = PausedOperation::start(&host, "before-prepare", "generate");
    state
        .maintain_producer_at(OP_PRODUCER, u64::MAX - 1)
        .unwrap();
    assert!(
        state
            .rooted_original(OP_PRODUCER, "instructions-publication-recovery")
            .unwrap()
            .is_some()
    );
    paused.interrupt();
    // Interruption is not a success observation, even after explicit recovery.
    chrono_instructions::maintain(&host.root).unwrap();
    let status = state.producer_status(OP_PRODUCER, None).unwrap();
    for record in status["records"].as_array().unwrap() {
        if record["outcome"]["publication"] == "not-started" {
            assert!(record["observed_return"].is_null());
        }
    }
}

#[test]
fn known_evaluator_error_and_interrupted_unknown_return_remain_distinct() {
    let host = Host::new();
    host.init().unwrap();
    test_support::generate_with_state_failure(&host.root, "before-result").unwrap_err();
    let before = chrono_instructions::status(&host.root, None).unwrap();
    let failure = before["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["observed_return"]["result"].get("error").is_some())
        .unwrap();
    assert!(failure["observed_return"]["process_exit"].is_null());
    chrono_instructions::maintain(&host.root).unwrap();
    PausedOperation::start(&host, "before-result", "generate").interrupt();
    chrono_instructions::maintain(&host.root).unwrap();
    let after = chrono_instructions::status(&host.root, None).unwrap();
    assert!(
        after["records"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["observed_return"].is_null()
                && r["outcome"]["publication"] == "verified-committed")
    );
}

#[test]
fn interrupted_creation_recovery_can_be_interrupted_before_prepare_again() {
    // Recovery requires the interrupted child's last metadata capability to be
    // closed; unrelated fixture forks must not briefly retain the same lock.
    if !isolated_capability::isolated_capability_test(
        "interrupted_creation_recovery_can_be_interrupted_before_prepare_again",
    ) {
        return;
    }
    let host = Host::new();
    PausedOperation::start(&host, "intent", "init").interrupt();
    PausedOperation::start(&host, "creation-recovered", "init").interrupt();
    chrono_instructions::maintain(&host.root).unwrap();
    assert_eq!(attempts(&host).len(), 1);
    assert_eq!(
        operation_json(&attempts(&host)[0], "result.json")["publication"],
        "not-started"
    );
    assert!(host.snapshot().is_empty());
}

#[test]
fn interrupted_backup_drift_is_preserved_and_blocks_recovery() {
    let host = Host::new();
    host.init().unwrap();
    host.write(METHOD, "next");
    PausedOperation::start(&host, "published-0", "generate").interrupt();
    let backup = attempts(&host)
        .into_iter()
        .map(|p| p.join("original-0"))
        .find(|p| p.exists())
        .unwrap();
    fs::write(&backup, b"independent original change").unwrap();
    let before = host.snapshot();
    assert!(chrono_instructions::maintain(&host.root).is_err());
    assert_eq!(fs::read(&backup).unwrap(), b"independent original change");
    assert_eq!(host.snapshot(), before);
}

#[test]
fn actual_state_publication_collisions_fail_and_preserve_pending_originals() {
    // Recovery checks the child's last metadata capability after it returns.
    // Run without unrelated fixture forks retaining that same capability.
    if !isolated_capability::isolated_capability_test(
        "actual_state_publication_collisions_fail_and_preserve_pending_originals",
    ) {
        return;
    }
    for name in ["result.json", "commit.json", "unexpected"] {
        let host = Host::new();
        host.init().unwrap();
        let before = host.read("CLAUDE.md");
        host.write(METHOD, "new publication");
        let point = if name == "result.json" {
            "before-result"
        } else {
            "prepared"
        };
        let paused = PausedOperation::start(&host, point, "generate");
        let pending = attempts(&host)
            .into_iter()
            .find(|p| !p.join("result.json").exists())
            .unwrap();
        let collision = pending.join(name);
        fs::create_dir(&collision).unwrap();
        let stderr = paused.finish_with_failure();
        assert!(!stderr.is_empty());
        assert!(
            inventory(&host)
                .rooted_original(OP_PRODUCER, "instructions-publication-recovery")
                .unwrap()
                .is_some()
        );
        assert!(chrono_instructions::maintain(&host.root).is_err());
        assert!(collision.is_dir());
        fs::remove_dir(collision).unwrap();
        chrono_instructions::maintain(&host.root).unwrap();
        assert_eq!(attempts(&host).len(), 2);
        if name == "commit.json" {
            assert_eq!(host.read("CLAUDE.md"), before);
        } else {
            assert!(
                String::from_utf8(host.read("CLAUDE.md"))
                    .unwrap()
                    .contains("new publication")
            );
        }
        assert_alias(&host);
    }
}

#[test]
fn external_input_observations_reject_content_identity_and_alias_drift_before_publication() {
    for drift in ["content", "identity", "alias"] {
        let host = Host::new();
        if drift == "alias" {
            let actual = host.temp.path().join("actual-method.md");
            fs::rename(&host.method, &actual).unwrap();
            std::os::unix::fs::symlink(&actual, &host.method).unwrap();
        }
        let paused = PausedOperation::start(&host, "prepared", "raw-init");
        match drift {
            "content" => fs::write(&host.method, "new external bytes").unwrap(),
            "identity" => {
                let bytes = fs::read(&host.method).unwrap();
                fs::rename(&host.method, host.temp.path().join("previous-external.md")).unwrap();
                fs::write(&host.method, bytes).unwrap();
            }
            "alias" => {
                let other = host.temp.path().join("other-method.md");
                fs::write(&other, fs::read(&host.method).unwrap()).unwrap();
                fs::remove_file(&host.method).unwrap();
                std::os::unix::fs::symlink(other, &host.method).unwrap();
            }
            _ => unreachable!(),
        }
        let stderr = paused.finish_with_failure();
        assert!(stderr.contains("external input changed"), "{stderr}");
        assert!(host.snapshot().is_empty());
        assert_eq!(attempts(&host).len(), 1);
        let result = operation_json(&attempts(&host)[0], "result.json");
        assert_eq!(result["publication"], "restored");
    }
}

#[test]
fn interrupted_staging_cleanup_continues_without_repeating_publication() {
    let host = Host::new();
    PausedOperation::start(&host, "prepared", "init").interrupt();
    PausedOperation::start(&host, "cleaned-next-0", "init").interrupt();
    chrono_instructions::maintain(&host.root).unwrap();
    assert_eq!(attempts(&host).len(), 1);
    assert!(host.snapshot().is_empty());
    assert_eq!(
        operation_json(&attempts(&host)[0], "result.json")["publication"],
        "restored"
    );
}
