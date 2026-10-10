#[path = "../../judge-filemap-tests/tests/support/mod.rs"]
mod support;
#[path = "support/temporary.rs"]
mod temporary;
use chrono_judge_projects::pairs;
use serde_json::json;
use std::{collections::BTreeSet, fs};
use support::*;

fn fixture(
    script: bool,
    implementation: &str,
    testing: &str,
) -> (chrono_worktree::TemporaryHost, Values) {
    let root = temporary::temporary_host("host λ ");
    let mut values = values();
    let registry = values.get_mut(PROJECTS).unwrap();
    registry["schema_version"] = json!(2);
    let production = json!({"execute":{"operation":"host.produce","tool":"sh","argv":[]}});
    let verification = json!({"execute":{"operation":"host.verify","tool":"sh","argv":[]}});
    if script {
        registry["projects"] = json!([]);
        registry["scripts"] = json!([
            {"id":"p","path":"sources/value.data","test_script":"t","language":implementation,"actions":production},
            {"id":"t","path":"assays/contract.opaque","tests_for":"p","language":testing,"actions":verification}
        ]);
    } else {
        registry["projects"] = json!([
            {"id":"p","kind":"production","test_project":"t","language":implementation,"actions":production},
            {"id":"t","kind":"test","tests_for":"p","language":testing,"actions":verification}
        ]);
    }
    let prefix = if script { "script" } else { "project" };
    let filemap = values.get_mut(FM).unwrap();
    for (name, owner) in [("sources/value.data", "p"), ("assays/contract.opaque", "t")] {
        let path = root.path().join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "opaque fixture input").unwrap();
        let mut entry = file(name, json!([]));
        entry["owner"] = json!(owner);
        filemap["files"].as_array_mut().unwrap().push(entry);
    }
    filemap["project_edges"] = json!([
        edge(&format!("{prefix}:p"), "test-execution", "test:t"),
        edge(&format!("{prefix}:t"), "test-execution", "test:t")
    ]);
    (root, values)
}

#[test]
fn same_declared_language_accepts_arbitrary_host_languages_and_paths() {
    for script in [false, true] {
        for language in [
            "rust",
            "go",
            "typescript",
            "python",
            "shell",
            "future host language",
        ] {
            let (root, values) = fixture(script, language, language);
            let prefix = if script { "script" } else { "project" };
            for node in [
                format!("{prefix}:p"),
                format!("{prefix}:t"),
                "test:t".into(),
            ] {
                pairs(root.path(), &load(&values), &BTreeSet::from([node])).unwrap();
            }
        }
    }
}

#[test]
fn shell_python_exception_is_explicit_and_directional() {
    for script in [false, true] {
        let prefix = if script { "script" } else { "project" };
        let affected = BTreeSet::from([format!("{prefix}:p")]);
        let (root, values) = fixture(script, "shell", "python");
        pairs(root.path(), &load(&values), &affected).unwrap();
        for (implementation, testing) in [
            ("python", "shell"),
            ("rust", "python"),
            ("go", "typescript"),
        ] {
            let (root, values) = fixture(script, implementation, testing);
            let error = pairs(root.path(), &load(&values), &affected).unwrap_err();
            assert!(error.starts_with("E_TEST_LANGUAGE:"), "{error}");
            assert!(error.contains(implementation) && error.contains(testing));
        }
    }
}

#[test]
fn language_only_delta_selects_the_pair_and_unrelated_delta_does_not_rejudge_it() {
    for script in [false, true] {
        let (root, old) = fixture(script, "rust", "rust");
        let mut new = old.clone();
        let collection = if script { "scripts" } else { "projects" };
        new.get_mut(PROJECTS).unwrap()[collection][1]["language"] = json!("python");
        let before = load(&old);
        let after = load(&new);
        let (impact, findings) =
            chrono_judge_filemap::produce(&before, &after, CONFIG, &[delta(PROJECTS)]);
        assert!(findings.is_empty(), "{findings:?}");
        assert_eq!(
            impact
                .required_tests
                .iter()
                .map(|t| t.node.as_str())
                .collect::<Vec<_>>(),
            ["test:t"]
        );
        let affected = impact.closure.reached.keys().cloned().collect();
        assert!(
            pairs(root.path(), &after, &affected)
                .unwrap_err()
                .starts_with("E_TEST_LANGUAGE:")
        );
        let (unrelated, findings) =
            chrono_judge_filemap::produce(&after, &after, CONFIG, &[delta("doc.txt")]);
        assert!(findings.is_empty(), "{findings:?}");
        assert!(unrelated.required_tests.is_empty());
        pairs(
            root.path(),
            &after,
            &unrelated.closure.reached.keys().cloned().collect(),
        )
        .unwrap();
    }
}

#[test]
fn language_v2_does_not_reinterpret_legacy_v1_pairs() {
    let (root, mut values) = fixture(false, "rust", "python");
    values.get_mut(PROJECTS).unwrap()["schema_version"] = json!(1);
    for row in values.get_mut(PROJECTS).unwrap()["projects"]
        .as_array_mut()
        .unwrap()
    {
        row.as_object_mut().unwrap().remove("language");
    }
    pairs(
        root.path(),
        &load(&values),
        &BTreeSet::from(["project:p".into()]),
    )
    .unwrap();
}
