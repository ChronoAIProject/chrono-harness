use super::*;
use serde_json::{Value, json};
const CATALOG: &str = ".chrono-harness/instructions/catalog.json";

fn value(host: &Host, path: &str) -> Value {
    serde_json::from_slice(&host.read(path)).unwrap()
}
fn put(host: &Host, path: &str, v: &Value) {
    host.write(path, serde_json::to_vec_pretty(v).unwrap());
}
fn atom(id: &str, requires: &[&str], text: &str) -> Value {
    json!({"id":id,"requires":requires,"variants":[{"locale":"en","source":{"type":"inline","text":text}}]})
}
fn fixture() -> Host {
    let host = Host::new();
    let result = cli_init(&host, None, None);
    assert_eq!(result.exit_code, 0, "{result:?}");
    let mut cat = value(&host, CATALOG);
    cat["atoms"] = json!([
        atom("base", &[], "base\r\n"),
        atom("left", &["base"], "left"),
        atom("right", &["base"], "right"),
        atom("top", &["left", "right"], "top")
    ]);
    put(&host, CATALOG, &cat);
    let mut manifest = value(&host, MANIFEST);
    manifest["outputs"] = json!([
        {"id":"root","path":"CLAUDE.md","format":"root-guide","locale":"en","roots":["top"]},
        {"id":"doc","path":"docs/nested/guide.md","format":"markdown","locale":"en","roots":["right","left","top"]}
    ]);
    put(&host, MANIFEST, &manifest);
    host
}

#[test]
fn diamond_composition_uses_declared_order_and_renders_shared_prerequisite_once() {
    let host = fixture();
    generate(&host.root).unwrap();
    let root = String::from_utf8(host.read("CLAUDE.md")).unwrap();
    let doc = String::from_utf8(host.read("docs/nested/guide.md")).unwrap();
    assert!(root.contains("base\r\n\n\nleft\n\nright\n\ntop"), "{root}");
    assert!(doc.contains("base\r\n\n\nright\n\nleft\n\ntop"), "{doc}");
    assert_eq!(root.matches("base\r\n").count(), 1);
    assert_eq!(doc.matches("base\r\n").count(), 1);
    assert!(generate(&host.root).unwrap().is_empty());
}

#[test]
fn selected_missing_translation_fails_before_any_output_write() {
    let host = fixture();
    let mut cat = value(&host, CATALOG);
    cat["atoms"][0]["variants"] = json!([]);
    put(&host, CATALOG, &cat);
    let before = host.snapshot();
    let err = generate(&host.root).unwrap_err();
    assert!(
        err.contains("root") && err.contains("base") && err.contains("en"),
        "{err}"
    );
    assert_eq!(host.snapshot(), before);
}

fn fails_unchanged(host: &Host, expected: &str) {
    let before = host.snapshot();
    let err = generate(&host.root).unwrap_err();
    assert!(err.contains(expected), "expected {expected:?}: {err}");
    assert_eq!(host.snapshot(), before, "{err}");
}

#[test]
fn graph_integrity_includes_unselected_atoms_and_diagnoses_chains() {
    for (kind, expected) in [
        ("cycle", "top -> left -> base -> top"),
        ("unselected-cycle", "orphan -> orphan"),
        ("missing-dependency", "unresolved atom lost"),
        ("missing-root", "output doc: unresolved atom lost"),
        ("duplicate-atom", "duplicate atom base"),
        ("duplicate-variant", "duplicate variant en"),
        ("duplicate-locale", "duplicate locale en"),
        ("unknown-locale", "unregistered locale xx"),
        ("duplicate-output", "duplicate output ID root"),
        ("duplicate-path", "duplicate output path CLAUDE.md"),
        ("extra-root", "exactly one root-guide"),
    ] {
        let host = fixture();
        let mut cat = value(&host, CATALOG);
        let mut m = value(&host, MANIFEST);
        match kind {
            "cycle" => {
                cat["atoms"].as_array_mut().unwrap().reverse();
                cat["atoms"][3]["requires"] = json!(["top"]);
            }
            "unselected-cycle" => {
                cat["atoms"]
                    .as_array_mut()
                    .unwrap()
                    .push(atom("orphan", &["orphan"], ""))
            }
            "missing-dependency" => cat["atoms"][0]["requires"] = json!(["lost"]),
            "missing-root" => m["outputs"][1]["roots"] = json!(["lost"]),
            "duplicate-atom" => {
                let duplicate = cat["atoms"][0].clone();
                cat["atoms"].as_array_mut().unwrap().push(duplicate);
            }
            "duplicate-variant" => {
                let v = cat["atoms"][0]["variants"][0].clone();
                cat["atoms"][0]["variants"].as_array_mut().unwrap().push(v);
            }
            "duplicate-locale" => {
                let v = cat["locales"][1].clone();
                cat["locales"].as_array_mut().unwrap().push(v);
            }
            "unknown-locale" => cat["atoms"][0]["variants"][0]["locale"] = json!("xx"),
            "duplicate-output" => m["outputs"][1]["id"] = json!("root"),
            "duplicate-path" => m["outputs"][1]["path"] = json!("CLAUDE.md"),
            "extra-root" => {
                m["outputs"].as_array_mut().unwrap().remove(0);
            }
            _ => unreachable!(),
        };
        put(&host, CATALOG, &cat);
        put(&host, MANIFEST, &m);
        fails_unchanged(&host, expected);
    }
}

#[test]
fn strict_json_rejects_duplicate_and_unknown_fields_in_nested_data() {
    for (path, needle, replacement) in [
        (
            CATALOG,
            "\"schema_version\": 1",
            "\"schema_version\": 1, \"schema_version\": 1",
        ),
        (
            CATALOG,
            "\"id\": \"base\"",
            "\"id\": \"base\", \"id\": \"base\"",
        ),
        (
            CATALOG,
            "\"text\": \"left\"",
            "\"text\": \"left\", \"text\": \"ignored\"",
        ),
        (
            CATALOG,
            "\"type\": \"inline\"",
            "\"type\": \"inline\", \"extra\": 0",
        ),
        (
            CATALOG,
            "\"type\": \"inline\"",
            "\"type\": \"inline\", \"type\": \"file\"",
        ),
        (
            CATALOG,
            "\"requires\": []",
            "\"requires\": [], \"unknown\": 0",
        ),
        (
            MANIFEST,
            "\"locale\": \"en\"",
            "\"locale\": \"en\", \"locale\": \"en\"",
        ),
    ] {
        let host = fixture();
        let original = String::from_utf8(host.read(path)).unwrap();
        let edited = original.replacen(needle, replacement, 1);
        assert_ne!(original, edited);
        host.write(path, edited);
        fails_unchanged(&host, "invalid");
    }
}

#[test]
fn explicit_third_locale_uses_its_own_frame_notice_and_exact_variants() {
    let host = fixture();
    let mut cat = value(&host, CATALOG);
    let mut m = value(&host, MANIFEST);
    cat["locales"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"test-ZZ","root_frame":"FRAME-ZZ","projection_notice":"NOTICE-ZZ"}));
    for (i, a) in cat["atoms"].as_array_mut().unwrap().iter_mut().enumerate() {
        a["variants"].as_array_mut().unwrap().push(
            json!({"locale":"test-ZZ","source":{"type":"inline","text":format!("ZZ-{i}\r\n")}}),
        );
    }
    // An unselected atom may lack translations entirely.
    cat["atoms"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"unused","requires":[],"variants":[]}));
    for o in m["outputs"].as_array_mut().unwrap() {
        o["locale"] = json!("test-ZZ");
    }
    put(&host, CATALOG, &cat);
    put(&host, MANIFEST, &m);
    generate(&host.root).unwrap();
    let root = String::from_utf8(host.read("CLAUDE.md")).unwrap();
    let doc = String::from_utf8(host.read("docs/nested/guide.md")).unwrap();
    assert!(
        root.contains("FRAME-ZZ\n")
            && root.contains("ZZ-0\r\n\n\nZZ-1\r\n\n\nZZ-2\r\n\n\nZZ-3\r\n")
    );
    assert!(
        doc.contains("NOTICE-ZZ\n") && doc.contains("ZZ-0\r\n\n\nZZ-2\r\n\n\nZZ-1\r\n\n\nZZ-3\r\n")
    );
    m["outputs"][1]["locale"] = json!("unregistered");
    put(&host, MANIFEST, &m);
    fails_unchanged(&host, "output doc: unregistered locale");
}

#[test]
fn file_variants_are_exact_shared_explicit_inputs_even_when_unselected() {
    let host = fixture();
    let mut cat = value(&host, CATALOG);
    let path = ".chrono-harness/rules/exact.md";
    let body = "\u{feff}file\r\nlast";
    host.write(path, body);
    cat["atoms"][0]["variants"][0]["source"] = json!({"type":"file","path":path});
    let mut unused = atom("unused", &[], "");
    unused["variants"][0]["source"] = json!({"type":"file","path":path});
    cat["atoms"].as_array_mut().unwrap().push(unused);
    put(&host, CATALOG, &cat);
    generate(&host.root).unwrap();
    assert!(
        String::from_utf8(host.read("CLAUDE.md"))
            .unwrap()
            .contains(&format!("{body}\n\nleft"))
    );
    assert_eq!(host.read(path), body.as_bytes());
    cat["atoms"].as_array_mut().unwrap().last_mut().unwrap()["variants"][0]["source"]["path"] =
        json!(".chrono-harness/rules/unselected.md");
    put(&host, CATALOG, &cat);
    fails_unchanged(&host, "unselected.md: registered source is missing");
    host.write(".chrono-harness/rules/unselected.md", [0xff]);
    fails_unchanged(&host, "expected UTF-8");
}

fn add_skill(m: &mut Value) {
    m["outputs"].as_array_mut().unwrap().push(json!({"id":"skill","path":"skills/repair/SKILL.md","format":"skill","locale":"en","roots":["left"],"skill":{"name":"repair","description":"Use when a failure repeats: inspect \"actual evidence\" and C:\\logs."}}));
}
#[test]
fn skills_start_with_yaml_and_preserve_declared_scalar_metadata() {
    let host = fixture();
    let mut m = value(&host, MANIFEST);
    add_skill(&mut m);
    put(&host, MANIFEST, &m);
    generate(&host.root).unwrap();
    let text = String::from_utf8(host.read("skills/repair/SKILL.md")).unwrap();
    assert!(text.starts_with("---\nname: \"repair\"\ndescription: "));
    let lines: Vec<_> = text.lines().collect();
    let name: String = serde_json::from_str(lines[1].strip_prefix("name: ").unwrap()).unwrap();
    let desc: String =
        serde_json::from_str(lines[2].strip_prefix("description: ").unwrap()).unwrap();
    assert_eq!(name, m["outputs"][2]["skill"]["name"]);
    assert_eq!(desc, m["outputs"][2]["skill"]["description"]);
    assert_eq!(lines[3], "---");
    assert!(lines[4].contains("id=skill format=skill begin"));
    assert!(text.contains("base\r\n\n\nleft\n"));
    assert!(!text.contains("\nright\n"));
    assert!(generate(&host.root).unwrap().is_empty());
    m["outputs"][2]["skill"]["description"] = json!("Updated declared purpose.");
    put(&host, MANIFEST, &m);
    assert_eq!(
        generate(&host.root).unwrap(),
        vec![
            fs::canonicalize(&host.root)
                .unwrap()
                .join("skills/repair/SKILL.md")
        ]
    );
}

#[test]
fn invalid_skill_metadata_and_format_bindings_fail_without_writes() {
    for (pointer, v) in [
        ("/skill/name", json!("Repair")),
        ("/skill/name", json!("-repair")),
        ("/skill/name", json!("repair--it")),
        ("/skill/name", json!("a".repeat(64))),
        ("/skill/name", json!("different")),
        ("/skill/description", json!(" ")),
        ("/skill/description", json!("line\nbreak")),
        ("/skill/description", json!("<tag>")),
        ("/skill/description", json!("a".repeat(1025))),
        ("/skill/description", json!("line\u{2028}break")),
        ("/path", json!("skills/repair/readme.md")),
        ("/format", json!("markdown")),
        ("/skill", Value::Null),
    ] {
        let host = fixture();
        let mut m = value(&host, MANIFEST);
        add_skill(&mut m);
        *m["outputs"][2]
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("{pointer}")) = v;
        put(&host, MANIFEST, &m);
        fails_unchanged(&host, "output skill");
    }
}

#[test]
fn extra_outputs_require_complete_matching_ownership_and_skill_frontmatter() {
    for kind in [
        "unowned",
        "id",
        "format",
        "producer",
        "end",
        "duplicate",
        "skill-prefix",
        "skill-header",
    ] {
        let host = fixture();
        let mut m = value(&host, MANIFEST);
        add_skill(&mut m);
        put(&host, MANIFEST, &m);
        generate(&host.root).unwrap();
        let path = if kind.starts_with("skill") {
            "skills/repair/SKILL.md"
        } else {
            "docs/nested/guide.md"
        };
        let old = String::from_utf8(host.read(path)).unwrap();
        let edited = match kind {
            "unowned" => "host-owned original".into(),
            "id" => old.replace("id=doc", "id=other"),
            "format" => old.replace("format=markdown", "format=skill"),
            "producer" => old.replace("producer=chrono-instructions", "producer=other"),
            "end" => old.replace(" end -->", " broken -->"),
            "duplicate" => format!("{old}<!-- chrono-instructions:extra -->"),
            "skill-prefix" => format!("\n{old}"),
            "skill-header" => old.replacen("name: \"repair\"", "name: [broken", 1),
            _ => unreachable!(),
        };
        host.write(path, edited);
        // Another output needs a change; ownership failures must precede all writes.
        let mut cat = value(&host, CATALOG);
        cat["atoms"][0]["variants"][0]["source"]["text"] = json!("changed");
        put(&host, CATALOG, &cat);
        fails_unchanged(&host, "unowned or malformed");
    }
}

#[test]
fn removed_and_renamed_outputs_remain_untouched_until_explicit_retirement() {
    let host = fixture();
    let mut m = value(&host, MANIFEST);
    add_skill(&mut m);
    put(&host, MANIFEST, &m);
    generate(&host.root).unwrap();
    let old_doc = host.read("docs/nested/guide.md");
    let old_skill = host.read("skills/repair/SKILL.md");
    m["outputs"].as_array_mut().unwrap().pop();
    m["outputs"][1]["path"] = json!("new/guide.md");
    put(&host, MANIFEST, &m);
    generate(&host.root).unwrap();
    assert_eq!(host.read("docs/nested/guide.md"), old_doc);
    assert_eq!(host.read("skills/repair/SKILL.md"), old_skill);
    assert_eq!(host.read("new/guide.md"), old_doc);
    assert!(generate(&host.root).unwrap().is_empty());
}

#[test]
fn source_output_and_file_ancestor_conflicts_are_rejected_before_writes() {
    for path in [
        "/absolute.md",
        "./doc.md",
        "docs//doc.md",
        "docs/../doc.md",
        "docs\\doc.md",
        "docs/\nfile.md",
        "AGENTS.md",
        CATALOG,
        CONTEXT,
        MANIFEST,
        ".chrono-harness",
        "CLAUDE.md/child",
        ".chrono-harness/instructions/catalog.json/child",
    ] {
        let host = fixture();
        let mut m = value(&host, MANIFEST);
        m["outputs"][1]["path"] = json!(path);
        put(&host, MANIFEST, &m);
        let before = host.snapshot();
        assert!(generate(&host.root).is_err(), "{path}");
        assert_eq!(host.snapshot(), before);
    }
    for path in [
        "outside.md",
        "../outside.md",
        MANIFEST,
        CATALOG,
        CONTEXT,
        ".chrono-harness",
    ] {
        let host = fixture();
        let mut cat = value(&host, CATALOG);
        cat["atoms"][0]["variants"][0]["source"] = json!({"type":"file","path":path});
        put(&host, CATALOG, &cat);
        let before = host.snapshot();
        assert!(generate(&host.root).is_err(), "{path}");
        assert_eq!(host.snapshot(), before);
    }
    let host = fixture();
    let mut m = value(&host, MANIFEST);
    m["catalog"] = json!(CONTEXT);
    put(&host, MANIFEST, &m);
    fails_unchanged(&host, "source/control overlap");
}

#[cfg(unix)]
#[test]
fn output_and_input_links_and_linked_ancestors_never_publish() {
    use std::os::unix::fs::symlink;
    for kind in [
        "output-symlink",
        "output-hardlink",
        "ancestor",
        "source-link",
        "source-hardlink",
        "file-ancestor",
    ] {
        let host = fixture();
        match kind {
            "ancestor" => symlink(host.temp.path(), host.root.join("docs")).unwrap(),
            "file-ancestor" => host.write("docs", "ordinary file"),
            "output-symlink" | "output-hardlink" => {
                fs::create_dir_all(host.root.join("docs/nested")).unwrap();
                if kind == "output-symlink" {
                    symlink(&host.method, host.root.join("docs/nested/guide.md")).unwrap();
                } else {
                    fs::hard_link(&host.method, host.root.join("docs/nested/guide.md")).unwrap();
                }
            }
            _ => {
                let path = ".chrono-harness/input.md";
                if kind == "source-link" {
                    symlink(&host.method, host.root.join(path)).unwrap();
                } else {
                    fs::hard_link(&host.method, host.root.join(path)).unwrap();
                }
                let mut cat = value(&host, CATALOG);
                cat["atoms"][0]["variants"][0]["source"] = json!({"type":"file","path":path});
                put(&host, CATALOG, &cat);
            }
        }
        let before = host.snapshot();
        assert!(generate(&host.root).is_err(), "{kind}");
        assert_eq!(host.snapshot(), before);
    }
}

#[test]
fn late_extra_output_failure_rolls_back_nested_directories_and_prior_outputs() {
    for after in [1, 2] {
        let host = fixture();
        let mut m = value(&host, MANIFEST);
        add_skill(&mut m);
        put(&host, MANIFEST, &m);
        let before = host.snapshot();
        let err = test_support::generate_with_failure(&host.root, after, None).unwrap_err();
        assert!(
            err.contains("injected publication failure") && err.contains("unrestored=[]"),
            "{err}"
        );
        assert_eq!(host.snapshot(), before, "{err}");
        generate(&host.root).unwrap();
        assert!(generate(&host.root).unwrap().is_empty());
    }
}

#[test]
fn incomplete_extra_output_rollback_reports_retained_file_and_allows_explicit_recovery() {
    let host = fixture();
    let mut m = value(&host, MANIFEST);
    add_skill(&mut m);
    put(&host, MANIFEST, &m);
    let root = host.read("CLAUDE.md");
    let err = test_support::generate_with_failure(&host.root, 2, Some(1)).unwrap_err();
    let path = fs::canonicalize(&host.root)
        .unwrap()
        .join("docs/nested/guide.md");
    assert!(err.contains(&format!("unrestored=[{path:?}]")), "{err}");
    assert_eq!(host.read("CLAUDE.md"), root);
    assert!(path.is_file());
    fs::remove_file(path).unwrap();
    generate(&host.root).unwrap();
    assert!(generate(&host.root).unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn both_known_legacy_identities_preserve_opaque_bytes_and_modes_under_init_and_generate() {
    use std::os::unix::fs::PermissionsExt;
    for v2 in [false, true] {
        for init_command in [false, true] {
            let host = legacy_host();
            let mut m = legacy_manifest();
            if v2 {
                m["render"] = json!("literal-core/relative-alias/v2");
                m["outputs"] = json!([{"role":"claude-guide","path":"CLAUDE.md"},{"role":"agents-relative-alias","path":"AGENTS.md"}]);
            }
            put(&host, MANIFEST, &m);
            let body = "\u{feff}opaque arbitrary language\r\nno final newline";
            host.write(METHOD, body);
            fs::set_permissions(host.root.join(METHOD), fs::Permissions::from_mode(0o640)).unwrap();
            let context = host.read(CONTEXT);
            if init_command {
                assert_eq!(cli_init(&host, None, None).exit_code, 0);
            } else {
                generate(&host.root).unwrap();
            }
            assert_eq!(host.read(METHOD), body.as_bytes());
            assert_eq!(host.read(CONTEXT), context);
            assert_eq!(
                fs::metadata(host.root.join(METHOD))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o640
            );
            let cat = value(&host, CATALOG);
            assert_eq!(
                cat["atoms"],
                json!([{"id":"legacy.method","requires":[],"variants":[{"locale":"und","source":{"type":"file","path":METHOD}}]}])
            );
            assert_eq!(value(&host, MANIFEST)["outputs"][0]["locale"], "und");
            assert!(generate(&host.root).unwrap().is_empty());
        }
    }
}

#[test]
fn init_locale_is_a_fresh_binding_and_registered_overrides_cannot_replace_atoms() {
    for method in [false, true] {
        let host = Host::new();
        let mut args = vec![
            "init".into(),
            "--host-root".into(),
            host.root.clone().into_os_string(),
            "--locale".into(),
            "en".into(),
        ];
        if method {
            args.extend(["--methodology".into(), host.method.clone().into_os_string()]);
        }
        let output = dispatch(&args);
        assert_eq!(output.exit_code, 0, "{output:?}");
        let m = value(&host, MANIFEST);
        assert_eq!(m["outputs"][0]["locale"], "en");
        if method {
            assert_eq!(host.read(METHOD), fs::read(&host.method).unwrap());
        }
        let before = host.snapshot();
        assert_eq!(dispatch(&args).exit_code, 0);
        assert_eq!(host.snapshot(), before);
        args[4] = "zh-CN".into();
        assert_eq!(dispatch(&args).exit_code, 1);
        assert_eq!(host.snapshot(), before);
        if !method {
            assert_eq!(cli_init(&host, Some(&host.method), None).exit_code, 1);
            assert_eq!(host.snapshot(), before);
        }
    }
    let host = Host::new();
    let r = dispatch(&[
        "init".into(),
        "--host-root".into(),
        host.root.clone().into_os_string(),
        "--locale".into(),
        "missing".into(),
    ]);
    assert_eq!(r.exit_code, 1);
    assert!(host.snapshot().is_empty());
}

#[cfg(target_os = "macos")]
#[test]
fn case_aliases_in_prospective_mac_paths_fail_before_publication() {
    for other in ["docs/nested/Guide.md", "DOCS/nested/other.md"] {
        let host = fixture();
        let mut m = value(&host, MANIFEST);
        let mut extra = m["outputs"][1].clone();
        extra["id"] = json!("other");
        extra["path"] = json!(other);
        m["outputs"].as_array_mut().unwrap().push(extra);
        put(&host, MANIFEST, &m);
        fails_unchanged(&host, "case alias");
    }
}

#[cfg(target_os = "macos")]
#[test]
fn unicode_casefold_aliases_in_prospective_mac_files_fail_before_writes() {
    rejects_prospective_unicode_casefold_aliases("docs/straße.md", "docs/STRASSE.md");
}

#[cfg(target_os = "macos")]
#[test]
fn unicode_casefold_aliases_in_prospective_mac_directories_fail_before_writes() {
    // Different leaf names still require one unambiguous spelling for the directory.
    rejects_prospective_unicode_casefold_aliases("docs/straße/goal.md", "docs/STRASSE/evidence.md");
}

#[cfg(target_os = "macos")]
fn rejects_prospective_unicode_casefold_aliases(first: &str, second: &str) {
    use std::os::unix::fs::MetadataExt;

    // Exercise both an existing alias and an alias that publication would create.
    for retained_alias in [true, false] {
        let host = Host::new();
        assert_eq!(cli_init(&host, None, None).exit_code, 0);
        if !retained_alias {
            fs::remove_file(host.root.join("AGENTS.md")).unwrap();
        }
        let mut m = value(&host, MANIFEST);
        m["outputs"][0]["title"] = json!("Pending root change");
        for (id, path, atom) in [
            ("street-original", first, "core.goal"),
            ("street-folded", second, "core.evidence"),
        ] {
            m["outputs"].as_array_mut().unwrap().push(json!({
                "id": id, "path": path, "format": "markdown", "locale": "en", "roots": [atom]
            }));
        }
        put(&host, MANIFEST, &m);
        assert!(!host.root.join("docs").exists());
        let before = host.snapshot();
        let identities = || {
            before
                .keys()
                .map(|path| {
                    let meta = fs::symlink_metadata(host.root.join(path)).unwrap();
                    (meta.ino(), meta.mtime(), meta.mtime_nsec())
                })
                .collect::<Vec<_>>()
        };
        let identities_before = identities();
        let result = dispatch(&[
            "generate".into(),
            "--host-root".into(),
            host.root.clone().into_os_string(),
        ]);
        assert_eq!(result.exit_code, 1, "{result:?}");
        assert!(result.stderr.contains("case alias"), "{result:?}");
        assert!(result.stderr.contains("straße"), "{result:?}");
        assert!(result.stderr.contains("STRASSE"), "{result:?}");
        assert_eq!(host.snapshot(), before);
        assert_eq!(identities(), identities_before);
    }
}

#[cfg(target_os = "macos")]
#[test]
fn unicode_normalization_aliases_in_new_mac_outputs_fail_before_writes() {
    let host = fixture();
    let mut m = value(&host, MANIFEST);
    m["outputs"][1]["path"] = json!("docs/café.md");
    let mut extra = m["outputs"][1].clone();
    extra["id"] = json!("other");
    extra["path"] = json!("docs/cafe\u{301}.md");
    m["outputs"].as_array_mut().unwrap().push(extra);
    put(&host, MANIFEST, &m);
    fails_unchanged(&host, "alias");
}

#[cfg(target_os = "macos")]
#[test]
fn distinct_unicode_paths_remain_supported_without_rewriting_registration() {
    let host = fixture();
    let mut m = value(&host, MANIFEST);
    m["outputs"][1]["path"] = json!("文档/café.md");
    let mut extra = m["outputs"][1].clone();
    extra["id"] = json!("other");
    extra["path"] = json!("文档/cafè.md");
    m["outputs"].as_array_mut().unwrap().push(extra);
    put(&host, MANIFEST, &m);
    let plan = host.read(MANIFEST);
    generate(&host.root).unwrap();
    assert!(host.root.join("文档/café.md").is_file());
    assert!(host.root.join("文档/cafè.md").is_file());
    assert_eq!(host.read(MANIFEST), plan);
    assert!(generate(&host.root).unwrap().is_empty());
}
