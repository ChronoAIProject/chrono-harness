use chrono_host_release::{
    execution::Environment,
    identity::{Channel, VerifiedCi, resolve},
};

const SOURCE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const TREE: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

fn delivery(channel: &str) -> Environment {
    [
        ("CHRONO_RELEASE_CHANNEL", channel),
        ("CHRONO_RELEASE_SOURCE", SOURCE),
        ("CHRONO_RELEASE_CI_RUN_ID", "1234"),
        ("CHRONO_RELEASE_CI_ATTEMPT", "2"),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}

#[test]
fn ordinary_build_uses_the_plan_and_observed_source() {
    let result = resolve("v0.1.0-beta.29", SOURCE, TREE, &Environment::new()).unwrap();
    assert_eq!(result.channel, Channel::Build);
    assert_eq!(result.version, "v0.1.0-beta.29");
    assert_eq!(result.source_commit, SOURCE);
    assert_eq!(result.source_tree, TREE);
    assert_eq!(result.verified_ci, None);
}

#[test]
fn preview_version_is_deterministic_and_binds_the_full_source_and_ci_attempt() {
    let environment = delivery("preview");
    let result = resolve("v0.1.0-beta.29", SOURCE, TREE, &environment).unwrap();
    assert_eq!(result.channel, Channel::Preview);
    assert_eq!(result.version, format!("v0.1.0-dev.1234.2.g{SOURCE}"));
    assert_eq!(
        result.verified_ci,
        Some(VerifiedCi {
            run_id: 1234,
            attempt: 2
        })
    );
    assert_eq!(
        result,
        resolve("v0.1.0-beta.29", SOURCE, TREE, &environment).unwrap()
    );
    for (key, value) in [
        ("CHRONO_RELEASE_CI_RUN_ID", "1235"),
        ("CHRONO_RELEASE_CI_ATTEMPT", "3"),
    ] {
        let mut changed = environment.clone();
        changed.insert(key.into(), value.into());
        assert_ne!(
            result.version,
            resolve("v0.1.0-beta.29", SOURCE, TREE, &changed)
                .unwrap()
                .version
        );
    }
    let other = "cccccccccccccccccccccccccccccccccccccccc";
    let mut changed = environment;
    changed.insert("CHRONO_RELEASE_SOURCE".into(), other.into());
    assert_ne!(
        result.version,
        resolve("v0.1.0-beta.29", other, TREE, &changed)
            .unwrap()
            .version
    );
}

#[test]
fn stable_requires_an_explicit_matching_final_version() {
    let mut environment = delivery("stable");
    assert!(resolve("v1.2.3", SOURCE, TREE, &environment).is_err());
    environment.insert("CHRONO_RELEASE_VERSION".into(), "v1.2.3".into());
    let result = resolve("v1.2.3", SOURCE, TREE, &environment).unwrap();
    assert_eq!(result.channel, Channel::Stable);
    assert_eq!(result.version, "v1.2.3");
    assert_eq!(result.verified_ci.unwrap().run_id, 1234);
    environment.insert("CHRONO_RELEASE_VERSION".into(), "v1.2.4".into());
    assert!(resolve("v1.2.3", SOURCE, TREE, &environment).is_err());
    environment.insert("CHRONO_RELEASE_VERSION".into(), "v1.2.3-beta.1".into());
    assert!(resolve("v1.2.3-beta.1", SOURCE, TREE, &environment).is_err());
}

#[test]
fn published_channels_reject_missing_mismatched_or_ambiguous_identity() {
    for key in [
        "CHRONO_RELEASE_SOURCE",
        "CHRONO_RELEASE_CI_RUN_ID",
        "CHRONO_RELEASE_CI_ATTEMPT",
    ] {
        let mut environment = delivery("preview");
        environment.remove(std::ffi::OsStr::new(key));
        assert!(
            resolve("v1.2.3", SOURCE, TREE, &environment).is_err(),
            "{key}"
        );
    }
    for key in ["CHRONO_RELEASE_CI_RUN_ID", "CHRONO_RELEASE_CI_ATTEMPT"] {
        for value in ["0", "01", "-1", "1x", "18446744073709551616"] {
            let mut environment = delivery("preview");
            environment.insert(key.into(), value.into());
            assert!(
                resolve("v1.2.3", SOURCE, TREE, &environment).is_err(),
                "{key}={value}"
            );
        }
    }
    let mut environment = delivery("preview");
    environment.insert("CHRONO_RELEASE_SOURCE".into(), "dev".into());
    assert!(resolve("v1.2.3", SOURCE, TREE, &environment).is_err());
    environment = delivery("preview");
    environment.insert("CHRONO_RELEASE_VERSION".into(), "v9.9.9".into());
    assert!(resolve("v1.2.3", SOURCE, TREE, &environment).is_err());
    assert!(resolve("v1.2.3", SOURCE, TREE, &delivery("unknown")).is_err());
    for version in ["1.2.3", "v1.2", "v01.2.3", "v1.two.3", "v1.2.3.4"] {
        assert!(
            resolve(version, SOURCE, TREE, &delivery("preview")).is_err(),
            "{version}"
        );
    }
}

#[test]
fn source_observation_must_be_complete_lowercase_oids() {
    for source in ["dev", "a", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"] {
        assert!(resolve("v1.2.3", source, TREE, &Environment::new()).is_err());
        assert!(resolve("v1.2.3", SOURCE, source, &Environment::new()).is_err());
    }
}
