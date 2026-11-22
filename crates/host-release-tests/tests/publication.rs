#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
use super::policy::native_delivery as delivery;
#[cfg(verus_keep_ghost)]
use super::policy::publication;
#[cfg(not(verus_keep_ghost))]
use chrono_host_release::delivery;
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {
#[cfg_attr(not(verus_keep_ghost), test)]
fn original_run_and_attempt_must_both_be_positive_and_canonical() {
    let valid = publication::publication_attempt(4567, 3);
    let absent_run = publication::publication_attempt(0, 3);
    let absent_attempt = publication::publication_attempt(4567, 0);
    let number = delivery::canonical_positive(3, true);
    let padded = delivery::canonical_positive(3, false);
    let zero = delivery::canonical_positive(0, true);
    if !(valid && !absent_run && !absent_attempt && number && !padded && !zero) { panic!(); }
}
}

#[cfg(not(verus_keep_ghost))]
use chrono_host_release::{
    identity::{Channel, Identity, VerifiedCi},
    publication::{self, Asset, Commands, Config, Reply},
};
#[cfg(not(verus_keep_ghost))]
use serde_json::{Value, json};
#[cfg(not(verus_keep_ghost))]
use std::collections::{BTreeMap, VecDeque};

#[cfg(not(verus_keep_ghost))]
fn config() -> Config {
    Config {
        repository: "example/product".into(),
        workflow_id: 99,
        workflow_path: ".github/workflows/ci.yml".into(),
        branch: "dev".into(),
        stable_branch: "main".into(),
    }
}
#[cfg(not(verus_keep_ghost))]
fn identity() -> Identity {
    Identity {
        channel: Channel::Preview,
        version: "v1.2.3-dev.1234.2.gaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
        source_commit: "a".repeat(40),
        source_tree: "b".repeat(40),
        verified_ci: Some(VerifiedCi {
            run_id: 1234,
            attempt: 2,
        }),
    }
}
#[cfg(not(verus_keep_ghost))]
fn assets() -> BTreeMap<String, Asset> {
    BTreeMap::from([(
        "install.sh".into(),
        Asset {
            path: "/outputs/install.sh".into(),
            sha256: "c".repeat(64),
            size: 18,
        },
    )])
}
#[cfg(not(verus_keep_ghost))]
struct Mock {
    identity: Identity,
    stable_branch: Value,
    ci: Value,
    release: Option<Value>,
    uploaded: Vec<Value>,
    tag: Option<String>,
    calls: Vec<Vec<String>>,
    bodies: Vec<Value>,
    upload_exits: VecDeque<i32>,
    omit_upload: bool,
    lookup: Option<Value>,
}
#[cfg(not(verus_keep_ghost))]
impl Mock {
    fn new() -> Self {
        Self {
            identity: identity(),
            stable_branch: Value::Null,
            ci: serde_json::from_str(include_str!("data/landing-ci.json")).unwrap(),
            release: None,
            uploaded: vec![],
            tag: None,
            calls: vec![],
            bodies: vec![],
            upload_exits: VecDeque::new(),
            omit_upload: false,
            lookup: None,
        }
    }
    fn draft(&mut self) {
        self.release = Some(
            json!({"id":7,"tag_name":self.identity.version,"target_commitish":self.identity.source_commit,"prerelease":self.identity.channel == Channel::Preview,"draft":true}),
        );
    }
    fn upload(&mut self, name: &str) {
        let (size, hash) = match name {
            "install.sh" => (18, "c".repeat(64)),
            "release.json" => (42, "d".repeat(64)),
            _ => panic!("unexpected asset: {name}"),
        };
        assert!(
            !self.uploaded.iter().any(|asset| asset["name"] == name),
            "duplicate upload: {name}"
        );
        self.uploaded.push(
            json!({"name":name,"size":size,"digest":format!("sha256:{hash}"),"state":"uploaded"}),
        );
    }
    fn published(&mut self) {
        self.draft();
        self.upload("install.sh");
        self.release.as_mut().unwrap()["draft"] = json!(false);
        self.tag = Some(self.identity.source_commit.clone());
    }
    fn mutated(&self) -> bool {
        self.calls.iter().any(|args| {
            args.get(1).is_some_and(|v| v == "upload")
                || (args.get(2).is_some_and(|v| v == "POST" || v == "PATCH")
                    && args[3] != "graphql")
        })
    }
}
#[cfg(not(verus_keep_ghost))]
impl Commands for Mock {
    fn body(&mut self, value: &Value) -> Result<String, String> {
        let i = self.bodies.len();
        self.bodies.push(value.clone());
        Ok(format!("body-{i}"))
    }
    fn execute(&mut self, program: &str, args: &[String]) -> Result<Reply, String> {
        assert_eq!(program, "gh");
        self.calls.push(args.to_vec());
        let not_found = || Reply {
            exit: 1,
            stdout: serde_json::to_vec(&json!({"message":"Not Found","status":"404"})).unwrap(),
        };
        let reply = |v: Value| {
            Ok(Reply {
                exit: 0,
                stdout: serde_json::to_vec(&v).unwrap(),
            })
        };
        if args[0] == "release" {
            let name = args[3]
                .strip_prefix("/outputs/")
                .expect("absolute asset path");
            assert_eq!(
                args,
                [
                    "release",
                    "upload",
                    &self.identity.version,
                    &format!("/outputs/{name}"),
                    "--repo",
                    "example/product"
                ]
            );
            let exit = self.upload_exits.pop_front().unwrap_or(0);
            if exit == 0 && !self.omit_upload {
                self.upload(name);
            }
            return Ok(Reply {
                exit,
                stdout: vec![],
            });
        }
        assert_eq!(&args[..2], ["api", "--method"]);
        let (method, path) = (args[2].as_str(), args[3].as_str());
        if path == "repos/example/product/actions/runs/1234/attempts/2" {
            assert_eq!(method, "GET");
            return reply(self.ci.clone());
        }
        if path == "repos/example/product/git/ref/heads/main" {
            assert_eq!(method, "GET");
            assert_eq!(args.len(), 4);
            return if self.stable_branch.is_null() {
                Ok(not_found())
            } else {
                reply(self.stable_branch.clone())
            };
        }
        if path.contains("/git/ref/tags/") {
            return match &self.tag {
                Some(sha) => reply(json!({"object":{"type":"commit","sha":sha}})),
                None => Ok(not_found()),
            };
        }
        if method == "POST" && path == "graphql" {
            assert_eq!(args.len(), 6);
            assert_eq!(args[4], "--input");
            let body = self.bodies.last().unwrap();
            assert_eq!(
                body["variables"],
                json!({"owner":"example","repo":"product","tag":self.identity.version})
            );
            assert!(
                body["query"]
                    .as_str()
                    .unwrap()
                    .contains("release(tagName:$tag){databaseId}")
            );
            let selected = self.release.as_ref().map(|_| json!({"databaseId":7}));
            return reply(
                self.lookup
                    .clone()
                    .unwrap_or_else(|| json!({"data":{"repository":{"release":selected}}})),
            );
        }
        if method == "GET" && path == "repos/example/product/releases/7" {
            return reply(self.release.clone().expect("existing release"));
        }
        if path.contains("/releases/tags/") {
            return match &self.release {
                Some(v) if v["draft"] == false => reply(v.clone()),
                _ => Ok(not_found()),
            };
        }
        if path == "repos/example/product/releases/7/assets?per_page=100" {
            assert_eq!(method, "GET");
            return reply(json!(self.uploaded));
        }
        if method == "POST" && path == "repos/example/product/releases" {
            assert_eq!(args.len(), 6);
            assert_eq!(args[4], "--input");
            let body = self.bodies.last().unwrap();
            assert_eq!(body["draft"], true);
            assert_eq!(body["target_commitish"], self.identity.source_commit);
            assert_eq!(
                body["prerelease"],
                self.identity.channel == Channel::Preview
            );
            assert!(self.release.is_none(), "created a second release");
            self.draft();
            return reply(self.release.clone().unwrap());
        }
        if method == "PATCH" && path == "repos/example/product/releases/7" {
            assert!(
                self.uploaded
                    .iter()
                    .any(|asset| asset["name"] == "install.sh")
            );
            assert_eq!(
                self.bodies.last().unwrap(),
                &json!({"draft":false,
                    "body":format!("Source: {}\nLanding CI: 1234 attempt 2\nRelease run: https://github.com/example/product/actions/runs/4567/attempts/3", self.identity.source_commit),
                    "make_latest":if self.identity.channel == Channel::Stable {"true"} else {"false"}})
            );
            self.release.as_mut().unwrap()["draft"] = json!(false);
            self.tag = Some(self.identity.source_commit.clone());
            return reply(self.release.clone().unwrap());
        }
        panic!("unexpected argv: {args:?}")
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn creates_a_draft_uploads_without_clobber_and_verifies_the_public_result() {
    let mut mock = Mock::new();
    let result =
        publication::publish(&config(), &identity(), &assets(), 4567, 3, &mut mock).unwrap();
    assert_eq!(result["release"]["draft"], false);
    assert_eq!(result["verified_ci"]["run_attempt"], 2);
    assert!(
        mock.calls
            .iter()
            .all(|args| !args.contains(&"--clobber".into()))
    );
    assert_eq!(mock.bodies.len(), 3);
    assert_eq!(
        mock.bodies[2]["body"],
        format!(
            "Source: {}\nLanding CI: 1234 attempt 2\nRelease run: https://github.com/example/product/actions/runs/4567/attempts/3",
            "a".repeat(40)
        )
    );
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn ordinary_build_cannot_call_the_publication_api() {
    let mut mock = Mock::new();
    let mut identity = identity();
    identity.channel = Channel::Build;
    identity.verified_ci = None;
    assert!(publication::publish(&config(), &identity, &assets(), 4567, 3, &mut mock).is_err());
    assert!(mock.calls.is_empty());
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn resumes_identical_drafts_and_reuses_identical_public_versions() {
    for public in [false, true] {
        let mut mock = Mock::new();
        mock.published();
        if !public {
            mock.release.as_mut().unwrap()["draft"] = json!(true);
        }
        publication::publish(&config(), &identity(), &assets(), 4567, 3, &mut mock).unwrap();
        assert!(
            mock.calls
                .iter()
                .all(|a| a.get(1).is_none_or(|v| v != "upload"))
        );
        assert_eq!(mock.mutated(), !public);
    }
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn rejects_wrong_or_unsuccessful_ci_before_any_release_mutation() {
    for (key, value) in [
        ("id", json!(1235)),
        ("run_attempt", json!(3)),
        ("workflow_id", json!(100)),
        ("path", json!(".github/workflows/other.yml")),
        ("event", json!("pull_request")),
        ("head_branch", json!("integration/test")),
        ("head_sha", json!("d".repeat(40))),
        ("status", json!("in_progress")),
        ("conclusion", json!("failure")),
        ("repository", json!({"full_name":"other/product"})),
    ] {
        let mut mock = Mock::new();
        mock.ci[key] = value;
        assert!(
            publication::publish(&config(), &identity(), &assets(), 4567, 3, &mut mock).is_err(),
            "{key}"
        );
        assert_eq!(mock.calls.len(), 1);
        assert!(!mock.mutated());
    }
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn refuses_conflicting_public_or_draft_assets_sources_and_tags() {
    for case in 0..7 {
        let mut mock = Mock::new();
        mock.published();
        match case {
            0 => mock.uploaded[0]["digest"] = json!(format!("sha256:{}", "d".repeat(64))),
            1 => mock.uploaded.clear(),
            2 => mock.uploaded[0]["name"] = json!("unregistered"),
            3 => mock.release.as_mut().unwrap()["target_commitish"] = json!("d".repeat(40)),
            4 => mock.tag = Some("d".repeat(40)),
            5 => {
                mock.release.as_mut().unwrap()["draft"] = json!(true);
                mock.uploaded[0]["size"] = json!(19);
            }
            6 => mock.release.as_mut().unwrap()["prerelease"] = json!(false),
            _ => unreachable!(),
        }
        assert!(
            publication::publish(&config(), &identity(), &assets(), 4567, 3, &mut mock).is_err(),
            "{case}"
        );
        assert!(!mock.mutated());
    }
}
#[cfg(not(verus_keep_ghost))]
#[test]
fn resuming_a_partial_draft_checks_its_tag_before_uploading() {
    for mut mock in [Mock::new(), stable_mock()] {
        mock.draft();
        mock.tag = Some("d".repeat(40));
        let identity = mock.identity.clone();
        let error =
            publication::publish(&config(), &identity, &assets(), 4567, 3, &mut mock).unwrap_err();
        assert!(
            error.contains("release tag points at another source"),
            "{error}"
        );
        assert!(!mock.mutated(), "calls={:?}", mock.calls);
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn interrupted_upload_resumes_the_same_untagged_draft_and_only_adds_missing_assets() {
    let mut files = assets();
    files.insert(
        "release.json".into(),
        Asset {
            path: "/outputs/release.json".into(),
            sha256: "d".repeat(64),
            size: 42,
        },
    );
    for mut mock in [Mock::new(), stable_mock()] {
        let identity = mock.identity.clone();
        mock.upload_exits.extend([0, 17]);
        let failure =
            publication::publish(&config(), &identity, &files, 4567, 2, &mut mock).unwrap_err();
        assert!(
            failure.contains("original asset upload exited 17: release.json"),
            "{failure}"
        );
        let draft = mock.release.as_ref().unwrap();
        assert_eq!(draft["draft"], true);
        let id = draft["id"].clone();
        assert!(mock.tag.is_none());
        assert_eq!(mock.uploaded.len(), 1);
        let original = mock.uploaded[0].clone();
        assert_eq!(original["name"], "install.sh");

        let retry_start = mock.calls.len();
        let result =
            publication::publish(&config(), &identity, &files, 4567, 3, &mut mock).unwrap();
        assert_eq!(result["release"]["id"], id);
        assert_eq!(result["release"]["draft"], false);
        assert_eq!(mock.uploaded.len(), 2);
        assert_eq!(mock.uploaded[0], original);
        let uploads: Vec<_> = mock.calls[retry_start..]
            .iter()
            .filter(|args| args[0] == "release")
            .map(|args| args[3].as_str())
            .collect();
        assert_eq!(uploads, ["/outputs/release.json"]);
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn lookup_errors_or_changed_ids_cannot_create_or_modify_a_release() {
    for response in [
        json!({"errors":[{"message":"lookup failed"}],"data":{"repository":{"release":null}}}),
        json!({"data":{"repository":null}}),
        json!({"data":{"repository":{}}}),
        json!({"data":{"repository":{"release":{}}}}),
        json!({"data":{"repository":{"release":{"databaseId":0}}}}),
    ] {
        let mut mock = Mock::new();
        mock.lookup = Some(response);
        assert!(
            publication::publish(&config(), &identity(), &assets(), 4567, 3, &mut mock).is_err()
        );
        assert!(!mock.mutated());
    }
    let mut mock = Mock::new();
    mock.draft();
    mock.release.as_mut().unwrap()["id"] = json!(8);
    let error =
        publication::publish(&config(), &identity(), &assets(), 4567, 3, &mut mock).unwrap_err();
    assert!(error.contains("release ID differs"), "{error}");
    assert!(!mock.mutated());
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn failed_or_incomplete_upload_never_publishes_the_draft() {
    for incomplete in [false, true] {
        let mut mock = Mock::new();
        mock.omit_upload = incomplete;
        mock.upload_exits.push_back(if incomplete { 0 } else { 17 });
        assert!(
            publication::publish(&config(), &identity(), &assets(), 4567, 3, &mut mock).is_err()
        );
        assert_eq!(mock.release.unwrap()["draft"], true);
        assert!(
            mock.calls
                .iter()
                .all(|a| a.get(2).is_none_or(|m| m != "PATCH"))
        );
    }
}

#[cfg(not(verus_keep_ghost))]
fn stable_mock() -> Mock {
    let mut mock = Mock::new();
    mock.identity.channel = Channel::Stable;
    mock.identity.version = "v1.2.3".into();
    mock.stable_branch = json!({
        "ref":"refs/heads/main",
        "object":{"type":"commit","sha":mock.identity.source_commit}
    });
    mock
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn stable_publication_requires_the_fixed_source_on_main() {
    let mut mock = stable_mock();
    let identity = mock.identity.clone();
    let result = publication::publish(&config(), &identity, &assets(), 4567, 3, &mut mock).unwrap();
    assert_eq!(result["release"]["prerelease"], false);
    assert_eq!(result["release_branch"], mock.stable_branch);
    assert_eq!(mock.bodies.last().unwrap()["make_latest"], "true");
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn stable_rejects_missing_different_or_invalid_main_before_mutating_a_release() {
    for branch in [
        Value::Null,
        json!({"ref":"refs/heads/main","object":{"type":"commit","sha":"d".repeat(40)}}),
        json!({"ref":"refs/heads/dev","object":{"type":"commit","sha":"a".repeat(40)}}),
        json!({"ref":"refs/heads/main","object":{"type":"tag","sha":"a".repeat(40)}}),
        json!({"ref":"refs/heads/main","object":{"type":"commit"}}),
    ] {
        let mut mock = stable_mock();
        mock.stable_branch = branch;
        let identity = mock.identity.clone();
        let result = publication::publish(&config(), &identity, &assets(), 4567, 3, &mut mock);
        assert!(
            result.is_err(),
            "branch={:?}, result={result:?}",
            mock.stable_branch
        );
        assert!(!mock.mutated(), "calls={:?}", mock.calls);
    }
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn preview_publication_uses_dev_ci_without_requiring_main() {
    let mut mock = Mock::new();
    let result =
        publication::publish(&config(), &identity(), &assets(), 4567, 3, &mut mock).unwrap();
    assert_eq!(result["release"]["prerelease"], true);
    assert!(
        mock.calls
            .iter()
            .all(|args| !args.iter().any(|arg| arg.contains("/git/ref/heads/")))
    );
}

#[cfg(not(verus_keep_ghost))]
#[test]
fn stable_branch_declaration_is_required_distinct_and_literal() {
    for branch in ["", "dev", "../main", "main?x=1"] {
        let mut config = config();
        config.stable_branch = branch.into();
        let mut mock = stable_mock();
        let identity = mock.identity.clone();
        assert!(
            publication::publish(&config, &identity, &assets(), 4567, 3, &mut mock).is_err(),
            "branch={branch:?}"
        );
        assert!(mock.calls.is_empty());
    }
    let mut declaration = serde_json::to_value(config()).unwrap();
    declaration.as_object_mut().unwrap().remove("stable_branch");
    assert!(serde_json::from_value::<Config>(declaration).is_err());
}
