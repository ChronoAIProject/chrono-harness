use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) const QUERY: &str = r#"query($owner:String!,$name:String!,$number:Int!,$head:GitObjectID!) {
  repository(owner:$owner,name:$name) {
    nameWithOwner
    pullRequest(number:$number) {
      number url state headRefOid headRefName baseRefName isDraft
      headRepository { nameWithOwner }
      mergeCommit { oid }
    }
    object(oid:$head) { ... on Commit {
      oid statusCheckRollup { contexts(first:100) {
        pageInfo { hasNextPage }
        nodes {
          __typename
          ... on CheckRun {
            databaseId name status conclusion
            checkSuite {
              databaseId
              commit { oid }
              checkRuns(first:100,filterBy:{checkType:LATEST}) {
                pageInfo { hasNextPage }
                nodes { databaseId }
              }
            }
          }
          ... on StatusContext { context state commit { oid } }
        }
      } }
    } }
  }
}"#;

pub(crate) fn field<'a>(value: &'a Value, pointer: &str) -> Result<&'a str, String> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("invalid remote field {pointer}"))
}

pub(crate) fn repository(value: &Value, expected: &str) -> Result<(), String> {
    if !value.is_object()
        || value
            .get("errors")
            .is_some_and(|e| !e.is_null() && e.as_array().is_none_or(|a| !a.is_empty()))
        || field(value, "/data/repository/nameWithOwner")? != expected
    {
        return Err("GitHub returned errors or a different repository".into());
    }
    Ok(())
}

pub(crate) fn sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

pub(crate) struct Snapshot {
    pub number: u64,
    pub url: String,
    pub head: String,
    pub branch: String,
    pub state: String,
    pub draft: bool,
    pub checks: Vec<Value>,
    pub outcome: &'static str,
}

pub(crate) fn parse(
    value: &Value,
    repo: &str,
    base: &str,
    number: u64,
    head: &str,
    required: &[String],
) -> Result<Snapshot, String> {
    repository(value, repo)?;
    let pr = &value["data"]["repository"]["pullRequest"];
    if pr["number"].as_u64() != Some(number)
        || field(pr, "/baseRefName")? != base
        || field(pr, "/headRepository/nameWithOwner")? != repo
    {
        return Err("PR number, base or head repository mismatch".into());
    }
    let observed = field(pr, "/headRefOid")?;
    if !sha(observed) {
        return Err("invalid PR head SHA".into());
    }
    let state = field(pr, "/state")?;
    if !["OPEN", "CLOSED", "MERGED"].contains(&state) {
        return Err("unknown PR state".into());
    }
    let url = field(pr, "/url")?;
    let expected_url = format!("https://github.com/{repo}/pull/{number}");
    if url != expected_url {
        return Err("PR URL mismatch (only github.com is supported)".into());
    }
    let mut result = Snapshot {
        number,
        url: url.into(),
        head: observed.into(),
        branch: field(pr, "/headRefName")?.into(),
        state: state.into(),
        draft: pr["isDraft"].as_bool().ok_or("invalid draft state")?,
        checks: vec![],
        outcome: "pending",
    };
    if observed != head {
        result.outcome = "stale";
        return Ok(result);
    }
    if state == "CLOSED" {
        result.outcome = "closed";
        return Ok(result);
    }
    let commit = &value["data"]["repository"]["object"];
    if field(commit, "/oid")? != head {
        return Err("check commit does not match expected head".into());
    }
    let rollup = commit
        .get("statusCheckRollup")
        .ok_or("missing check rollup field")?;
    let mut current = BTreeMap::new();
    let mut ids = BTreeSet::new();
    let mut suites = BTreeMap::new();
    if !rollup.is_null() {
        let contexts = &rollup["contexts"];
        complete(contexts)?;
        for node in contexts["nodes"].as_array().ok_or("invalid check nodes")? {
            let (name, state) = match field(node, "/__typename")? {
                "CheckRun" => {
                    let id = node["databaseId"]
                        .as_u64()
                        .filter(|id| *id > 0)
                        .ok_or("invalid check ID")?;
                    if !ids.insert(id) {
                        return Err("duplicate check ID".into());
                    }
                    let name = field(node, "/name")?;
                    let status = field(node, "/status")?;
                    let state = match status {
                        "COMPLETED" => field(node, "/conclusion")?,
                        "QUEUED" | "IN_PROGRESS" | "WAITING" | "REQUESTED" | "PENDING" => {
                            if !node.get("conclusion").is_some_and(Value::is_null) {
                                return Err("pending check has a conclusion".into());
                            }
                            status
                        }
                        _ => return Err("unknown check status".into()),
                    };
                    classify(state)?;
                    if field(node, "/checkSuite/commit/oid")? != head {
                        return Err("check suite commit mismatch".into());
                    }
                    let latest = &node["checkSuite"]["checkRuns"];
                    complete(latest)?;
                    let mut latest_ids = BTreeSet::new();
                    for row in latest["nodes"].as_array().ok_or("invalid latest checks")? {
                        let latest_id = row["databaseId"]
                            .as_u64()
                            .filter(|id| *id > 0)
                            .ok_or("invalid latest check ID")?;
                        if !latest_ids.insert(latest_id) {
                            return Err("duplicate latest check ID".into());
                        }
                    }
                    if latest_ids.is_empty() {
                        return Err("empty latest check set".into());
                    }
                    let suite_id = node["checkSuite"]["databaseId"]
                        .as_u64()
                        .filter(|id| *id > 0)
                        .ok_or("invalid check suite ID")?;
                    if suites
                        .insert(suite_id, latest_ids.clone())
                        .is_some_and(|previous| previous != latest_ids)
                    {
                        return Err("inconsistent latest check set".into());
                    }
                    if !latest_ids.contains(&id) {
                        continue;
                    }
                    (name, state)
                }
                "StatusContext" => {
                    if field(node, "/commit/oid")? != head {
                        return Err("status commit mismatch".into());
                    }
                    let state = field(node, "/state")?;
                    if !["SUCCESS", "FAILURE", "ERROR", "PENDING", "EXPECTED"].contains(&state) {
                        return Err("unknown commit status".into());
                    }
                    (field(node, "/context")?, state)
                }
                _ => return Err("unknown check context type".into()),
            };
            if name.is_empty() || current.insert(name, state).is_some() {
                return Err("empty or ambiguous check name".into());
            }
        }
    }
    if suites.values().any(|latest| !latest.is_subset(&ids)) {
        return Err("latest check missing from complete rollup".into());
    }
    let mut failed = false;
    let mut waiting = false;
    for name in required {
        let state = current.get(name.as_str()).copied().unwrap_or("MISSING");
        match classify(state)? {
            "failed" => failed = true,
            "pending" => waiting = true,
            _ => {}
        }
        result.checks.push(json!({"name":name,"state":state}));
    }
    result.outcome = if failed {
        "failed"
    } else if waiting {
        "pending"
    } else {
        "passed"
    };
    Ok(result)
}

fn complete(value: &Value) -> Result<(), String> {
    if value
        .pointer("/pageInfo/hasNextPage")
        .and_then(Value::as_bool)
        != Some(false)
    {
        return Err("missing or truncated check page; complete facts required".into());
    }
    Ok(())
}

fn classify(state: &str) -> Result<&'static str, String> {
    match state {
        "SUCCESS" => Ok("passed"),
        "MISSING" | "QUEUED" | "IN_PROGRESS" | "WAITING" | "REQUESTED" | "PENDING" | "EXPECTED" => {
            Ok("pending")
        }
        "FAILURE" | "ERROR" | "CANCELLED" | "TIMED_OUT" | "ACTION_REQUIRED" | "STARTUP_FAILURE"
        | "STALE" | "NEUTRAL" | "SKIPPED" => Ok("failed"),
        _ => Err(format!("unknown check conclusion {state}")),
    }
}
