#[path = "support/recipe.rs"]
mod fixture;
use chrono_host_release::recipe::{Loaded, Scope};
use fixture::Fixture;
use serde_json::{Value, json};
use std::{fs, os::unix::fs::symlink};

#[test]
fn explicit_empty_and_subset_dependencies_keep_original_actions_and_staging() {
    let f = Fixture::new();
    let loaded = Loaded::load(f.root()).unwrap();
    assert!(loaded.unit("test_none").unwrap().needs().is_empty());
    assert_eq!(loaded.unit("test_a").unwrap().needs(), ["build_a"]);
    assert_eq!(
        loaded.actions["test.none"].argv,
        ["verify none", "space value", "$literal", "λ"]
    );
    assert_eq!(
        loaded
            .staging
            .iter()
            .map(|r| (&*r.asset, &*r.source, &*r.destination))
            .collect::<Vec<_>>(),
        [("a", "out/a", "consumer/a"), ("b", "out/b", "consumer/b")]
    );
    assert_eq!(loaded.contracts["test_a"]["release_assets"], json!(["a"]));
    assert_eq!(loaded.contracts["test_none"]["status"], "declared");
}

#[test]
fn native_frontends_have_exact_platforms_distinct_jobs_and_disjoint_paths() {
    for variant in 0..12 {
        let mut f = Fixture::new();
        match variant {
            0 => { f.config.as_object_mut().unwrap().remove("native_frontends"); }
            1 => f.config["native_frontends"] = json!({}),
            2 => f.config["native_frontends"]["unknown"] = f.config["native_frontends"]["fixture"].clone(),
            3 => f.config["native_frontends"]["fixture"]["job"] = json!("native_build_a"),
            4 => f.config["native_frontends"]["fixture"]["job"] = json!("invalid job"),
            5 => f.config["native_frontends"]["fixture"]["cost"] = json!(" "),
            6 => f.config["native_frontends"]["fixture"]["directory"] = json!("handoffs/a/nested"),
            7 => f.config["native_frontends"]["fixture"]["directory"] = json!("consumer"),
            8 => f.config["native_frontends"]["fixture"]["executable"] = json!("../escape"),
            9 => f.config["native_frontends"]["fixture"]["bootstrap_report"] = json!("chrono-host-release"),
            10 => f.config["native_frontends"]["fixture"]["bootstrap_profile"] = json!("profile.json"),
            11 => {
                f.config["native_jobs"]["second"] = f.config["native_jobs"]["fixture"].clone();
                f.config["native_frontends"]["second"] = f.config["native_frontends"]["fixture"].clone();
            }
            _ => unreachable!(),
        }
        f.save();
        assert!(Loaded::load(f.root()).is_err(), "variant {variant}");
        assert!(!f.root().join("frontend").exists());
        assert!(!f.root().join("handoffs").exists());
    }
}

#[test]
fn collection_and_non_rust_verification_only_resolve_their_actual_tools() {
    let f = Fixture::new();
    let loaded = Loaded::load(f.root()).unwrap();
    let collect = loaded.tools(f.root(), Scope::Collect).unwrap();
    assert_eq!(
        collect.keys().map(String::as_str).collect::<Vec<_>>(),
        ["git"]
    );
    let verify = loaded.tools(f.root(), Scope::Unit("test_none")).unwrap();
    assert_eq!(
        verify.keys().map(String::as_str).collect::<Vec<_>>(),
        ["git", "probe"]
    );
    assert!(loaded.tools(f.root(), Scope::Unit("unknown")).is_err());
    fs::create_dir(f.root().join("a")).unwrap();
    fs::write(f.root().join("a/Cargo.toml"), "selected manifest").unwrap();
    let build = loaded.tools(f.root(), Scope::Unit("build_a")).unwrap_err();
    assert!(build.contains("unavailable-release-"), "{build}");
    assert!(
        loaded
            .tools(f.root(), Scope::Unit("build_b"))
            .unwrap_err()
            .contains("missing release manifest")
    );
}

#[test]
fn booleans_cannot_be_null_numbers_strings_or_containers() {
    for invalid in [
        Value::Null,
        json!(0),
        json!(1),
        json!("false"),
        json!([]),
        json!({}),
    ] {
        for field in ["rust_incremental", "rust_toolchain"] {
            let mut f = Fixture::new();
            if field == "rust_incremental" {
                f.config[field] = invalid.clone();
            } else {
                f.config["units"][2][field] = invalid.clone();
            }
            f.save();
            assert!(Loaded::load(f.root()).is_err(), "{field}={invalid}");
        }
    }
    let mut f = Fixture::new();
    f.config.as_object_mut().unwrap().remove("rust_incremental");
    f.save();
    assert_eq!(
        Loaded::load(f.root()).unwrap().recipe.rust_incremental,
        None
    );
    f.config["rust_incremental"] = json!(false);
    f.save();
    assert_eq!(
        Loaded::load(f.root()).unwrap().recipe.rust_incremental,
        Some(false)
    );
}

#[test]
fn dependency_and_membership_mutations_are_rejected_before_tool_resolution() {
    for variant in 0..14 {
        let mut f = Fixture::new();
        match variant {
            0 => f.config["units"][2]["needs"] = json!(["build_a"]),
            1 => f.config["units"][3]["needs"] = json!([]),
            2 => f.config["units"][3]["needs"] = json!(["build_a", "build_a"]),
            3 => f.config["units"][3]["needs"] = json!(["missing"]),
            4 => f.config["units"][3]["needs"] = json!(["test_none"]),
            5 => f.config["collector"]["needs"] = json!(["build_a", "build_b", "test_a"]),
            6 => {
                f.config["collector"]["needs"] =
                    json!(["build_a", "build_b", "test_a", "test_a", "test_none"])
            }
            7 => f.config["units"][1]["asset"] = json!("a"),
            8 => f.config["units"][3]["operation"] = json!("test.none"),
            9 => f.config["unit_costs"]
                .as_object_mut()
                .unwrap()
                .remove("test_a")
                .map(|_| ())
                .unwrap(),
            10 => f.config["native_jobs"]["fixture"]["test_a"] = json!("native_test_none"),
            11 => f.config["local_workers"] = json!(0),
            12 => f.config["limits"]["unit_seconds"] = json!(0),
            13 => f.config["units"][0]["needs"] = json!(["build_b"]),
            _ => unreachable!(),
        }
        f.save();
        assert!(Loaded::load(f.root()).is_err(), "variant {variant}");
        assert!(!f.root().join("consumer").exists());
        assert!(!f.root().join("handoffs").exists());
    }
}

#[test]
fn staging_and_failure_evidence_require_owned_nonoverlapping_literal_paths() {
    for variant in 0..10 {
        let mut f = Fixture::new();
        match variant {
            0 => f.config["consumer_staging"]["bindings"][0]["destinations"] = json!(["../escape"]),
            1 => {
                f.config["consumer_staging"]["bindings"][0]["destinations"] =
                    json!(["out/a/nested"])
            }
            2 => {
                f.config["consumer_staging"]["bindings"][1]["destinations"] =
                    json!(["consumer/a/nested"])
            }
            3 => f.host["artifacts"][0]["tracked"] = json!(true),
            4 => f.host["artifacts"]
                .as_array_mut()
                .unwrap()
                .push(json!({"path":"consumer/a","tracked":false})),
            5 => f.config["units"][0]["handoff"] = json!("consumer"),
            6 => f.config["failure_evidence"]["test.a"] = json!(["failures/test-a/", "failures/"]),
            7 => f.config["failure_evidence"]["test.a"] = json!(["unowned/"]),
            8 => f.config["failure_evidence"]["test.a"] = json!(["failures/test-a"]),
            9 => f.config["failure_evidence"]["unknown"] = json!(["failures/test-a/"]),
            _ => unreachable!(),
        }
        f.save();
        assert!(Loaded::load(f.root()).is_err(), "variant {variant}");
    }
    let f = Fixture::new();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), f.root().join("failures")).unwrap();
    assert!(Loaded::load(f.root()).unwrap_err().contains("symlink"));
}

#[test]
fn ambiguous_original_actions_and_duplicate_json_members_are_rejected() {
    let mut f = Fixture::new();
    f.projects["scripts"][0]["actions"]["duplicate"] =
        f.projects["scripts"][0]["actions"]["none"].clone();
    f.save();
    assert!(Loaded::load(f.root()).unwrap_err().contains("ambiguous"));
    let f = Fixture::new();
    let file = f.root().join(".chrono-harness/release/build.json");
    let raw = fs::read_to_string(&file).unwrap();
    fs::write(&file, raw.replacen("{", "{\"schema\":\"forged\",", 1)).unwrap();
    assert!(
        Loaded::load(f.root())
            .unwrap_err()
            .contains("duplicate JSON member")
    );
}

#[test]
fn absent_consumer_explanation_is_visible_and_cannot_claim_verified_necessity() {
    let mut f = Fixture::new();
    f.config
        .as_object_mut()
        .unwrap()
        .remove("verification_consumers");
    f.save();
    let loaded = Loaded::load(f.root()).unwrap();
    assert_eq!(loaded.contracts["test_a"]["status"], "unverified");
    assert_eq!(
        loaded.contracts["test_a"]["findings"],
        json!(["W_RELEASE_CONSUMPTION_UNVERIFIED"])
    );
    assert_eq!(loaded.contracts["test_none"]["findings"], json!([]));
}
