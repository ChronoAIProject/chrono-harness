use chrono_instructions::{dispatch, generate, init, test_support};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

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
                let kind = fs::symlink_metadata(&path).unwrap().file_type();
                let value = if kind.is_symlink() {
                    format!("link:{:?}", fs::read_link(&path).unwrap()).into_bytes()
                } else if kind.is_dir() {
                    walk(root, &path, files);
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
        host.write("CLAUDE.md", bytes);
        let before = host.snapshot();
        assert!(host.init().unwrap_err().contains("managed delimiters"));
        let result = cli_init(&host, None, None);
        assert_eq!(result.exit_code, 1, "{result:?}");
        assert!(result.stderr.contains("managed delimiters"));
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
fn reordered_legacy_source_or_output_registration_is_invalid() {
    for field in ["sources", "outputs"] {
        let host = legacy_host();
        let mut value: serde_json::Value = serde_json::from_slice(&host.read(MANIFEST)).unwrap();
        value[field].as_array_mut().unwrap().reverse();
        host.write(MANIFEST, serde_json::to_vec(&value).unwrap());
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
    assert!(!host.root.join(".chrono-harness").exists());
    let backups: Vec<_> = fs::read_dir(&host.root)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".chrono-instructions-")
        })
        .collect();
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

fn legacy_manifest() -> serde_json::Value {
    serde_json::json!({
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
    })
}

fn legacy_host() -> Host {
    let host = Host::new();
    host.write(METHOD, fs::read(&host.method).unwrap());
    host.write(CONTEXT, b"deliberate nonempty context\r\n");
    fs::write(&host.context, host.read(CONTEXT)).unwrap();
    host.write(MANIFEST, serde_json::to_vec(&legacy_manifest()).unwrap());
    for name in ["AGENTS.md", "CLAUDE.md"] {
        host.write(
            name,
            format!("host preface\n{BEGIN}\nold reading route\n{END}\nhost tail"),
        );
    }
    host
}

#[test]
fn exact_v1_upgrades_through_init_or_generate_without_updating_adopted_sources() {
    for init_command in [false, true] {
        let host = legacy_host();
        let method = host.read(METHOD);
        let context = host.read(CONTEXT);
        let result = dispatch(&[
            if init_command {
                "init".into()
            } else {
                "generate".into()
            },
            "--host-root".into(),
            host.root.clone().into_os_string(),
        ]);
        assert_eq!(result.exit_code, 0, "{result:?}");
        assert!(result.stdout.contains("4 file(s) changed"));
        assert_eq!(host.read(METHOD), method);
        assert_eq!(host.read(CONTEXT), context);
        assert_alias(&host);
        let mut expected = b"host preface\n".to_vec();
        let block = expected_guide(&method);
        expected.extend_from_slice(&block[..block.len() - 1]);
        expected.extend_from_slice(b"\nhost tail");
        assert_eq!(host.read("CLAUDE.md"), expected);
        let manifest: serde_json::Value = serde_json::from_slice(&host.read(MANIFEST)).unwrap();
        assert_eq!(manifest["render"], "atomic-rules/relative-alias/v3");
        host.write(MANIFEST, serde_json::to_vec(&manifest).unwrap());
        let before = host.snapshot();
        assert!(host.init().unwrap().is_empty());
        assert!(generate(&host.root).unwrap().is_empty());
        assert_eq!(host.snapshot(), before); // retain current manifest formatting
    }
}

#[test]
fn corrupt_or_mixed_v1_is_not_migration_authority() {
    for pointer in [
        "/producer",
        "/render",
        "/sources/0/role",
        "/outputs/1/role",
        "/outputs/0/path",
    ] {
        let host = legacy_host();
        let mut value = legacy_manifest();
        *value.pointer_mut(pointer).unwrap() = serde_json::json!("unknown");
        host.write(MANIFEST, serde_json::to_vec(&value).unwrap());
        let before = host.snapshot();
        assert!(generate(&host.root).unwrap_err().contains("registration"));
        assert!(host.init().unwrap_err().contains("registration"));
        assert_eq!(host.snapshot(), before);
    }
    let host = legacy_host();
    fs::write(&host.method, "different explicitly selected input").unwrap();
    let before = host.snapshot();
    assert!(host.init().unwrap_err().contains("init inputs differ"));
    assert_eq!(host.snapshot(), before);
}

#[cfg(unix)]
#[test]
fn existing_alias_inode_survives_edits_and_migration() {
    use std::os::unix::fs::{MetadataExt, symlink};
    let host = legacy_host();
    fs::remove_file(host.root.join("AGENTS.md")).unwrap();
    symlink("CLAUDE.md", host.root.join("AGENTS.md")).unwrap();
    let identity = || {
        let m = fs::symlink_metadata(host.root.join("AGENTS.md")).unwrap();
        (m.dev(), m.ino(), m.modified().unwrap())
    };
    let before = identity();
    assert_eq!(generate(&host.root).unwrap().len(), 3);
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

#[cfg(unix)]
#[test]
fn upgrade_failure_after_alias_restores_old_registration_roots_and_modes() {
    use std::os::unix::fs::PermissionsExt;
    let host = legacy_host();
    for (path, mode) in [
        ("AGENTS.md", 0o640),
        ("CLAUDE.md", 0o600),
        (MANIFEST, 0o644),
    ] {
        fs::set_permissions(host.root.join(path), fs::Permissions::from_mode(mode)).unwrap();
    }
    let before = host.snapshot();
    let failure = test_support::init_with_failure(&host.root, &host.method, &host.context, 3, None)
        .unwrap_err();
    assert!(failure.contains("unrestored=[]"));
    assert_eq!(host.snapshot(), before);
    for (path, mode) in [
        ("AGENTS.md", 0o640),
        ("CLAUDE.md", 0o600),
        (MANIFEST, 0o644),
    ] {
        let meta = fs::symlink_metadata(host.root.join(path)).unwrap();
        assert!(meta.file_type().is_file());
        assert_eq!(meta.permissions().mode() & 0o777, mode);
    }
    assert_eq!(generate(&host.root).unwrap().len(), 4);
    assert_alias(&host);
    assert_eq!(
        fs::metadata(host.root.join("CLAUDE.md"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
}

#[test]
fn incomplete_fresh_alias_rollback_names_unrestored_link_and_allows_explicit_recovery() {
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
    let before = host.snapshot();
    assert!(host.init().is_err());
    assert_eq!(host.snapshot(), before);
    fs::remove_file(path).unwrap();
    host.init().unwrap();
    assert_alias(&host);
}

#[path = "atomic.rs"]
mod atomic;
