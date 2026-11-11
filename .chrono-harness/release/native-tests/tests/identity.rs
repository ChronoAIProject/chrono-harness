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

const WORKFLOW: &str = ".github/workflows/chrono-release.yml";
const WORKFLOW_COMMIT: &str = "cccccccccccccccccccccccccccccccccccccccc";
const BLOB: &str = "dddddddddddddddddddddddddddddddddddddddd";

fn workflow_environment(commit: &str) -> Environment {
    [
        ("GITHUB_REPOSITORY", "owner/repository"),
        ("GITHUB_WORKFLOW_SHA", commit),
        (
            "GITHUB_WORKFLOW_REF",
            "owner/repository/.github/workflows/chrono-release.yml@refs/heads/dev",
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.into(), v.into()))
    .collect()
}

#[derive(Default)]
struct WorkflowCommands {
    replies: std::collections::VecDeque<Result<chrono_host_release::publication::Reply, String>>,
    calls: Vec<(String, Vec<String>)>,
}

impl WorkflowCommands {
    fn blob(&mut self, blob: &str) {
        self.replies
            .push_back(Ok(chrono_host_release::publication::Reply {
                exit: 0,
                stdout: format!("{blob}\n").into_bytes(),
            }));
    }
}

impl chrono_host_release::publication::Commands for WorkflowCommands {
    fn execute(
        &mut self,
        program: &str,
        args: &[String],
    ) -> Result<chrono_host_release::publication::Reply, String> {
        self.calls.push((program.into(), args.to_vec()));
        self.replies.pop_front().expect("unexpected command")
    }

    fn body(&mut self, _: &serde_json::Value) -> Result<String, String> {
        panic!("workflow identity must not write an API request")
    }
}

fn expected_blob_query(commit: &str) -> (String, Vec<String>) {
    (
        "git".into(),
        vec![
            "--no-replace-objects".into(),
            "rev-parse".into(),
            "--verify".into(),
            "--end-of-options".into(),
            format!("{commit}:{WORKFLOW}"),
        ],
    )
}

#[test]
fn workflow_from_the_source_commit_needs_only_one_fixed_blob_read() {
    let mut commands = WorkflowCommands::default();
    commands.blob(BLOB);
    let result = chrono_host_release::identity::verify_workflow(
        WORKFLOW,
        SOURCE,
        &workflow_environment(SOURCE),
        &mut commands,
    )
    .unwrap();
    assert_eq!(result.workflow_commit, SOURCE);
    assert_eq!(result.source_commit, SOURCE);
    assert_eq!(result.path, WORKFLOW);
    assert_eq!(result.blob, BLOB);
    assert_eq!(commands.calls, vec![expected_blob_query(SOURCE)]);
}

#[test]
fn unrelated_branch_progress_is_allowed_when_the_workflow_blob_is_unchanged() {
    let mut commands = WorkflowCommands::default();
    commands.blob(BLOB);
    commands.blob(BLOB);
    let result = chrono_host_release::identity::verify_workflow(
        WORKFLOW,
        SOURCE,
        &workflow_environment(WORKFLOW_COMMIT),
        &mut commands,
    )
    .unwrap();
    assert_eq!(result.workflow_commit, WORKFLOW_COMMIT);
    assert_eq!(result.source_commit, SOURCE);
    assert_eq!(result.blob, BLOB);
    assert_eq!(
        commands.calls,
        vec![
            expected_blob_query(WORKFLOW_COMMIT),
            expected_blob_query(SOURCE)
        ]
    );
}

#[test]
fn changed_workflow_is_rejected_with_both_fixed_identities() {
    let mut commands = WorkflowCommands::default();
    commands.blob(BLOB);
    commands.blob(TREE);
    let error = chrono_host_release::identity::verify_workflow(
        WORKFLOW,
        SOURCE,
        &workflow_environment(WORKFLOW_COMMIT),
        &mut commands,
    )
    .unwrap_err();
    for expected in [
        "E_RELEASE_IDENTITY",
        "workflow",
        WORKFLOW_COMMIT,
        SOURCE,
        WORKFLOW,
    ] {
        assert!(error.contains(expected), "{error}");
    }
    assert_eq!(commands.calls.len(), 2);
}

#[test]
fn missing_or_mismatched_github_bindings_fail_before_any_command() {
    for key in [
        "GITHUB_REPOSITORY",
        "GITHUB_WORKFLOW_SHA",
        "GITHUB_WORKFLOW_REF",
    ] {
        let mut environment = workflow_environment(WORKFLOW_COMMIT);
        environment.remove(std::ffi::OsStr::new(key));
        let mut commands = WorkflowCommands::default();
        assert!(
            chrono_host_release::identity::verify_workflow(
                WORKFLOW,
                SOURCE,
                &environment,
                &mut commands,
            )
            .is_err(),
            "{key}"
        );
        assert!(commands.calls.is_empty());
    }
    for (key, value) in [
        ("GITHUB_WORKFLOW_SHA", "dev"),
        (
            "GITHUB_WORKFLOW_SHA",
            "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
        ),
        ("GITHUB_REPOSITORY", "other/repository"),
        (
            "GITHUB_WORKFLOW_REF",
            "owner/repository/.github/workflows/other.yml@refs/heads/dev",
        ),
        (
            "GITHUB_WORKFLOW_REF",
            "owner/repository/.github/workflows/chrono-release.yml@",
        ),
    ] {
        let mut environment = workflow_environment(WORKFLOW_COMMIT);
        environment.insert(key.into(), value.into());
        let mut commands = WorkflowCommands::default();
        assert!(
            chrono_host_release::identity::verify_workflow(
                WORKFLOW,
                SOURCE,
                &environment,
                &mut commands,
            )
            .is_err(),
            "{key}={value}"
        );
        assert!(commands.calls.is_empty());
    }
    for path in [
        "../workflow.yml",
        ".github/workflows/../other.yml",
        "workflow.yml",
        "",
    ] {
        let mut commands = WorkflowCommands::default();
        assert!(
            chrono_host_release::identity::verify_workflow(
                path,
                SOURCE,
                &workflow_environment(WORKFLOW_COMMIT),
                &mut commands,
            )
            .is_err(),
            "{path}"
        );
        assert!(commands.calls.is_empty());
    }
}

#[test]
fn git_failure_timeout_and_incomplete_blob_observations_cannot_pass() {
    use chrono_host_release::publication::Reply;
    for reply in [
        Err("original Git timed out".into()),
        Ok(Reply {
            exit: 128,
            stdout: format!("{BLOB}\n").into_bytes(),
        }),
        Ok(Reply {
            exit: -9,
            stdout: format!("{BLOB}\n").into_bytes(),
        }),
        Ok(Reply {
            exit: 0,
            stdout: vec![],
        }),
        Ok(Reply {
            exit: 0,
            stdout: vec![255],
        }),
        Ok(Reply {
            exit: 0,
            stdout: format!("{BLOB}\n{BLOB}\n").into_bytes(),
        }),
        Ok(Reply {
            exit: 0,
            stdout: format!("{BLOB}").into_bytes(),
        }),
        Ok(Reply {
            exit: 0,
            stdout: b"HEAD\n".to_vec(),
        }),
    ] {
        let mut commands = WorkflowCommands::default();
        commands.replies.push_back(reply);
        assert!(
            chrono_host_release::identity::verify_workflow(
                WORKFLOW,
                SOURCE,
                &workflow_environment(WORKFLOW_COMMIT),
                &mut commands,
            )
            .is_err()
        );
        assert_eq!(commands.calls, vec![expected_blob_query(WORKFLOW_COMMIT)]);
    }
}
