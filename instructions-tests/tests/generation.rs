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
fn init_retains_exact_inputs_and_both_entrypoints_route_to_readable_shared_sources() {
    let host = Host::new();
    let changed = host.init().unwrap();
    assert_eq!(changed.len(), 5);
    assert_eq!(host.read(METHOD), fs::read(&host.method).unwrap());
    assert_eq!(host.read(CONTEXT), b"");
    for name in ["AGENTS.md", "CLAUDE.md"] {
        let route = String::from_utf8(host.read(name)).unwrap();
        assert!(route.contains("read BOTH canonical files in full, in this order"));
        assert!(route.find(METHOD).unwrap() < route.find(CONTEXT).unwrap());
        for relative in [METHOD, CONTEXT] {
            assert!(host.root.join(relative).is_file());
        }
        assert!(route.contains("does not activate harness judges"));
    }
    let manifest: serde_json::Value = serde_json::from_slice(&host.read(MANIFEST)).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["producer"], "chrono-instructions/0.1.0");
    assert_eq!(manifest["render"], "read-both/v1");
    assert_eq!(manifest["sources"][0]["path"], METHOD);
    assert_eq!(manifest["sources"][1]["path"], CONTEXT);
    assert_eq!(manifest["outputs"][0]["path"], "AGENTS.md");
    assert_eq!(manifest["outputs"][1]["path"], "CLAUDE.md");
}

#[test]
fn repeated_init_and_generate_do_not_write_even_after_canonical_edits() {
    let host = Host::new();
    host.init().unwrap();
    let before = host.snapshot();
    let times: Vec<_> = [METHOD, CONTEXT, MANIFEST, "AGENTS.md", "CLAUDE.md"]
        .iter()
        .map(|p| fs::metadata(host.root.join(p)).unwrap().modified().unwrap())
        .collect();
    assert!(host.init().unwrap().is_empty());
    assert!(generate(&host.root).unwrap().is_empty());
    assert_eq!(host.snapshot(), before);
    for (path, time) in [METHOD, CONTEXT, MANIFEST, "AGENTS.md", "CLAUDE.md"]
        .iter()
        .zip(times)
    {
        assert_eq!(
            fs::metadata(host.root.join(path))
                .unwrap()
                .modified()
                .unwrap(),
            time
        );
    }
    host.write(METHOD, "自主修订的通用方法\n");
    host.write(CONTEXT, "项目事实：仅使用显式登记。\n");
    let edited = host.snapshot();
    assert!(generate(&host.root).unwrap().is_empty());
    assert_eq!(host.snapshot(), edited);
    assert!(host.init().unwrap_err().contains("init inputs differ"));
    assert_eq!(host.snapshot(), edited);
    // Explicit selection of the current canonical materials is also safe.
    assert!(
        init(
            &host.root,
            &host.root.join(METHOD),
            &host.root.join(CONTEXT)
        )
        .unwrap()
        .is_empty()
    );
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
    host.write("CLAUDE.md", b"unrelated without newline");
    host.write(".chrono-harness/keep.json", b"host configuration");
    host.init().unwrap();
    let agents = host.read("AGENTS.md");
    assert!(agents.starts_with(prefix));
    assert!(agents.ends_with(suffix));
    assert!(!String::from_utf8_lossy(&agents).contains("stale route"));
    assert!(
        host.read("CLAUDE.md")
            .starts_with(b"unrelated without newline\n")
    );
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
    fs::remove_file(host.root.join("CLAUDE.md")).unwrap();
    assert_eq!(generate(&host.root).unwrap().len(), 2);
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
            "\"schema_version\": 1",
            "\"schema_version\": 1, \"schema_version\": 1",
            1,
        ),
        original.replacen("\"schema_version\": 1", "\"schema_version\": 2", 1),
        original.replacen(
            "\"schema_version\": 1",
            "\"extra\": true, \"schema_version\": 1",
            1,
        ),
        original.replace("read-both/v1", "read-both/v9"),
        original.replace("chrono-instructions/0.1.0", "different-producer"),
        original.replace(METHOD, "../outside.md"),
        original.replace("host-context\"", "methodology\""),
        original.replace("CLAUDE.md", "AGENTS.md"),
        original.replacen("\"role\":", "\"extra\": 1, \"role\":", 1),
        original.replacen(
            "\"role\": \"methodology\"",
            "\"role\": \"methodology\", \"role\": \"methodology\"",
            1,
        ),
        format!("{original} true"),
    ];
    for bytes in cases {
        host.write(MANIFEST, bytes);
        let before = host.snapshot();
        let result = generate(&host.root);
        assert!(result.unwrap_err().contains("registration"));
        let result = cli_init(&host, None, None);
        assert_eq!(result.exit_code, 1, "{result:?}");
        assert!(result.stderr.contains("registration"));
        assert_eq!(host.snapshot(), before);
    }
}

#[test]
fn reordered_source_or_output_registration_is_invalid() {
    for field in ["sources", "outputs"] {
        let host = Host::new();
        host.init().unwrap();
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
    for path in [METHOD, CONTEXT] {
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
fn generate_reads_both_retained_sources_even_if_routes_are_current() {
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
    assert_eq!(init(&host.root, &selected, &host.context).unwrap().len(), 4);
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
    assert!(host.init().unwrap_err().contains("hard-linked"));
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
    for after in 0..5 {
        let host = Host::new();
        host.write("AGENTS.md", b"original agents\xff");
        host.write("CLAUDE.md", "original claude");
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
    host.write("AGENTS.md", "original agents");
    host.write("CLAUDE.md", "original claude");
    let failure =
        test_support::init_with_failure(&host.root, &host.method, &host.context, 4, Some(3))
            .unwrap_err();
    let expected = format!(
        "unrestored=[{:?}]",
        fs::canonicalize(&host.root).unwrap().join("AGENTS.md")
    );
    assert!(failure.contains(&expected), "{failure}");
    assert!(failure.contains("original retained at"));
    assert_eq!(host.read("CLAUDE.md"), b"original claude");
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
    assert_eq!(fs::read(&backups[0]).unwrap(), b"original agents");
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
    test_support::init_with_failure(&host.root, &host.method, &host.context, 4, None).unwrap_err();
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
    assert!(output.stdout.contains("5 file(s) changed"));
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
fn default_init_ships_exact_full_asset_empty_context_and_both_readable_routes() {
    let host = Host::new();
    let asset = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/methodology.md");
    host.write("AGENTS.md", b"existing agents\r\n");
    host.write("CLAUDE.md", b"existing claude without newline");
    host.write(".chrono-harness/unrelated.json", b"keep this");
    // Normal adoption needs no external input files at all.
    fs::remove_file(&host.method).unwrap();
    fs::remove_file(&host.context).unwrap();
    let result = cli_init(&host, None, None);
    assert_eq!(result.exit_code, 0, "{result:?}");
    assert!(result.stdout.contains("5 file(s) changed"));
    assert_eq!(host.read(METHOD), fs::read(&asset).unwrap());
    assert_eq!(host.read(CONTEXT), b"");
    for name in ["AGENTS.md", "CLAUDE.md"] {
        let route = String::from_utf8(host.read(name)).unwrap();
        assert!(route.contains("read BOTH canonical files in full, in this order"));
        assert!(route.find(METHOD).unwrap() < route.find(CONTEXT).unwrap());
        for path in [METHOD, CONTEXT] {
            fs::read_to_string(host.root.join(path)).unwrap();
        }
    }
    assert!(host.read("AGENTS.md").starts_with(b"existing agents\r\n"));
    assert!(
        host.read("CLAUDE.md")
            .starts_with(b"existing claude without newline\n")
    );
    assert_eq!(host.read(".chrono-harness/unrelated.json"), b"keep this");
}

#[test]
fn init_overrides_are_independent_and_retain_exact_selected_bytes() {
    let asset = Path::new(env!("CARGO_MANIFEST_DIR")).join("../assets/methodology.md");
    for (method, context) in [(true, false), (false, true), (true, true)] {
        let host = Host::new();
        fs::write(&host.context, "\u{feff}deliberate host facts\r\n").unwrap();
        let result = cli_init(
            &host,
            method.then_some(&host.method),
            context.then_some(&host.context),
        );
        assert_eq!(result.exit_code, 0, "{result:?}");
        assert_eq!(
            host.read(METHOD),
            fs::read(if method { &host.method } else { &asset }).unwrap()
        );
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
                host.write(METHOD, "customized method after installation\r\n");
                host.write(CONTEXT, "customized context after installation");
                let manifest: serde_json::Value =
                    serde_json::from_slice(&host.read(MANIFEST)).unwrap();
                host.write(MANIFEST, serde_json::to_vec(&manifest).unwrap());
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
