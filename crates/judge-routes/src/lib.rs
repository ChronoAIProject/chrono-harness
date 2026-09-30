//! Routes owns method planning and comparison with runner-produced observations.
use chrono_harness::{
    ProcessResult, facts,
    observation::{self, Tool},
    wire::{self, Finding, Request, Response, Status},
};
use chrono_judge_registration::{
    Registrations,
    execution::{Method, Plan},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Operation {
    pub method: Method,
    pub predecessors: BTreeSet<String>,
    pub timeout_seconds: u64,
    pub output_limit_bytes: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    pub schema: String,
    pub identity: String,
    pub binding: Value,
    pub root: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub selected: BTreeMap<String, Plan>,
    pub operations: Vec<Operation>,
    pub tools: BTreeMap<String, Tool>,
}
impl Execution {
    fn digest(&self) -> Result<String, String> {
        let mut value = serde_json::to_value(self).map_err(|e| e.to_string())?;
        value.as_object_mut().unwrap().remove("identity");
        wire::digest(&value)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "chrono-execution-plan/v1" || self.identity != self.digest()? {
            return Err("E_PLAN_IDENTITY".into());
        }
        Ok(())
    }
}
pub fn binding(req: &Request, inputs: &Value) -> Value {
    json!({"base":req.base.commit,"base_tree":req.base.tree,"candidate":req.candidate.commit,"candidate_tree":req.candidate.tree,"registry":req.registries.digest,"context":req.context.sha256,"inputs":inputs,"entry":req.observations["entry"],"run":req.observations["run"]})
}
pub fn expand(argv: &[String], binding: &Value) -> Vec<String> {
    argv.iter()
        .map(|a| match a.as_str() {
            "{base}" => binding["base"].as_str().unwrap_or("").into(),
            "{candidate}" => binding["candidate"].as_str().unwrap_or("").into(),
            _ => a.clone(),
        })
        .collect()
}
/// Merge explicit sequences into a DAG before observing tools or launching any command.
pub fn order(
    selected: &BTreeSet<String>,
    plans: &BTreeMap<String, Plan>,
    methods: &BTreeMap<String, Vec<Method>>,
    execute: &BTreeMap<String, String>,
) -> Result<(BTreeMap<String, Plan>, Vec<Operation>), String> {
    let mut operations: BTreeMap<String, Operation> = BTreeMap::new();
    let mut chosen = BTreeMap::new();
    for test in selected {
        let plan = plans
            .get(test)
            .ok_or_else(|| format!("E_ROUTE_MISSING: execution plan {test}"))?;
        if plan.operations.is_empty()
            || plan.operations.iter().collect::<BTreeSet<_>>().len() != plan.operations.len()
            || plan.timeout_seconds == 0
            || plan.output_limit_bytes == 0
            || plan.output_limit_bytes > 64 * 1024 * 1024
        {
            return Err(format!("E_EXECUTION_PLAN: {test}"));
        }
        let action = execute
            .get(test)
            .ok_or_else(|| format!("E_ROUTE_MISSING: unique execute action for {test}"))?;
        if !plan.operations.contains(action) {
            return Err(format!("E_EXECUTION_PLAN: {test} omits its execute action"));
        }
        let mut previous = None;
        for id in &plan.operations {
            let defs = methods
                .get(id)
                .ok_or_else(|| format!("E_ROUTE_MISSING: {id}"))?;
            if defs.len() != 1 || id == "validate.delta" {
                return Err(format!("E_ROUTE_AMBIGUOUS: {id}"));
            }
            let op = operations.entry(id.clone()).or_insert_with(|| Operation {
                method: defs[0].clone(),
                predecessors: BTreeSet::new(),
                timeout_seconds: plan.timeout_seconds,
                output_limit_bytes: plan.output_limit_bytes,
            });
            if op.timeout_seconds != plan.timeout_seconds
                || op.output_limit_bytes != plan.output_limit_bytes
            {
                return Err(format!("E_PLAN_BOUNDS: inconsistent shared operation {id}"));
            }
            if let Some(p) = previous {
                op.predecessors.insert(p);
            }
            previous = Some(id.clone());
        }
        chosen.insert(test.clone(), plan.clone());
    }
    let mut ordered = vec![];
    let mut done = BTreeSet::new();
    while !operations.is_empty() {
        let id = operations
            .iter()
            .find(|(_, op)| op.predecessors.is_subset(&done))
            .map(|(id, _)| id.clone())
            .ok_or("E_PLAN_CYCLE: inconsistent operation ordering")?;
        ordered.push(operations.remove(&id).unwrap());
        done.insert(id);
    }
    Ok((chosen, ordered))
}
pub fn prepare(
    root: &Path,
    binding: Value,
    selected: &BTreeSet<String>,
    plans: &BTreeMap<String, Plan>,
    methods: &BTreeMap<String, Vec<Method>>,
    execute: &BTreeMap<String, String>,
    declarations: &Value,
    environment: BTreeMap<String, String>,
    reused: &BTreeMap<String, Tool>,
) -> Result<Execution, String> {
    prepare_inner(
        root,
        binding,
        selected,
        plans,
        methods,
        execute,
        declarations,
        environment,
        reused,
        true,
    )
}
/// Legacy CI supplies its explicitly scoped declarations; missing version contracts remain unverified.
pub fn prepare_scoped(
    root: &Path,
    binding: Value,
    selected: &BTreeSet<String>,
    plans: &BTreeMap<String, Plan>,
    methods: &BTreeMap<String, Vec<Method>>,
    execute: &BTreeMap<String, String>,
    declarations: &Value,
    environment: BTreeMap<String, String>,
    reused: &BTreeMap<String, Tool>,
) -> Result<Execution, String> {
    prepare_inner(
        root,
        binding,
        selected,
        plans,
        methods,
        execute,
        declarations,
        environment,
        reused,
        false,
    )
}
fn prepare_inner(
    root: &Path,
    binding: Value,
    selected: &BTreeSet<String>,
    plans: &BTreeMap<String, Plan>,
    methods: &BTreeMap<String, Vec<Method>>,
    execute: &BTreeMap<String, String>,
    declarations: &Value,
    environment: BTreeMap<String, String>,
    reused: &BTreeMap<String, Tool>,
    strict: bool,
) -> Result<Execution, String> {
    let (selected, mut operations) = order(selected, plans, methods, execute)?;
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut tools = BTreeMap::new();
    for op in &mut operations {
        op.method.argv = expand(&op.method.argv, &binding);
        let id = &op.method.tool;
        if tools.contains_key(id) {
            continue;
        }
        let defs: Vec<_> = declarations
            .as_array()
            .ok_or("E_ROUTE_MISSING: tool declarations")?
            .iter()
            .filter(|t| t["id"] == *id)
            .collect();
        if defs.len() != 1 {
            return Err(format!("E_ROUTE_AMBIGUOUS: tool {id}"));
        }
        let t = defs[0];
        let program = t["program"].as_str().ok_or("tool program")?;
        let argv: Vec<String> =
            serde_json::from_value(t["version_argv"].clone()).map_err(|e| e.to_string())?;
        let tool = if let Some(tool) = reused.get(id) {
            tool.clone()
        } else {
            observation::tool(
                &root,
                program,
                &argv,
                &environment,
                op.timeout_seconds,
                op.output_limit_bytes,
            )?
        };
        if strict || t["expected_version"].is_string() {
            validate_tool(
                &tool,
                program,
                &argv,
                &environment,
                &root,
                t["expected_version"].as_str(),
            )?;
        }
        tools.insert(id.clone(), tool);
    }
    let mut plan = Execution {
        schema: "chrono-execution-plan/v1".into(),
        identity: String::new(),
        binding,
        root,
        environment,
        selected,
        operations,
        tools,
    };
    plan.identity = plan.digest()?;
    Ok(plan)
}
pub fn validate_tool(
    tool: &Tool,
    program: &str,
    argv: &[String],
    env: &BTreeMap<String, String>,
    root: &Path,
    expected: Option<&str>,
) -> Result<(), String> {
    let actual = &tool.version;
    let mut invocation = vec![tool.path.to_str().ok_or("tool path")?.to_string()];
    invocation.extend_from_slice(argv);
    if tool.program != program
        || !tool.path.is_absolute()
        || tool.path.file_name().and_then(|s| s.to_str()) != Some(&tool.basename)
        || actual.executable != tool.path
        || actual.sha256 != tool.sha256
        || actual.argv != invocation
        || actual.cwd != root
        || actual.environment != *env
        || actual.environment_digest != wire::digest(env)?
        || actual.failure.is_some()
        || actual.exit_code != 0
        || expected.is_none()
        || expected != Some(actual.stdout.trim_end())
        || actual.stdout_sha256 != chrono_harness::sha256(&actual.stdout_bytes)
        || actual.stdout.as_bytes() != actual.stdout_bytes
        || fs::read(&tool.path)
            .map(|b| chrono_harness::sha256(&b))
            .ok()
            .as_ref()
            != Some(&tool.sha256)
    {
        return Err("E_TOOL_BINDING: path/version/digest/environment observation mismatch".into());
    }
    Ok(())
}
pub fn validate_invocation(
    root: &Path,
    entry: &Value,
    template: &[String],
    binding: &Value,
) -> Result<(), String> {
    let argv: Vec<String> = serde_json::from_value(entry["argv"].clone())
        .map_err(|_| "E_ROUTE_MISSING: observed entry argv")?;
    let cwd = Path::new(
        entry["cwd"]
            .as_str()
            .ok_or("E_ROUTE_MISSING: observed cwd")?,
    );
    let expected = expand(template, binding);
    if argv.len() != expected.len() || argv.is_empty() {
        return Err("E_ROUTE_MISSING: canonical argv differs".into());
    }
    // Equivalent absolute paths allow the same registered method from another caller cwd.
    for (i, (a, b)) in argv.iter().zip(&expected).enumerate() {
        let path_arg =
            i == 0 || i > 0 && matches!(expected[i - 1].as_str(), "--config" | "--context");
        if path_arg {
            let actual = fs::canonicalize(if i > 0 && expected[i - 1] == "--context" {
                root.join(a)
            } else {
                cwd.join(a)
            })
            .map_err(|e| format!("E_ROUTE_MISSING: {e}"))?;
            let want =
                fs::canonicalize(root.join(b)).map_err(|e| format!("E_ROUTE_MISSING: {e}"))?;
            if actual != want {
                return Err("E_ROUTE_MISSING: canonical path differs".into());
            }
        } else if a != b {
            return Err("E_ROUTE_MISSING: canonical argv differs".into());
        }
    }
    Ok(())
}
pub fn canonical(req: &Request, r: &Registrations) -> Result<(), String> {
    let expected: Vec<String> =
        serde_json::from_value(r.config()["canonical_check"]["argv"].clone())
            .map_err(|e| e.to_string())?;
    validate_invocation(
        &req.candidate.root,
        &req.observations["entry"],
        &expected,
        &json!({"base":req.base.commit,"candidate":req.candidate.commit}),
    )?;
    if r.config()["schema_version"] == 4 {
        chrono_harness::prepared::validate_binding(
            &req.candidate.root,
            r.config(),
            &req.config_path,
            Some(&req.base.commit),
            &req.candidate.commit,
            false,
            &None,
            Some(&req.context),
            &req.observations["preparation"],
        )?;
    }
    if chrono_judge_registration::execution::methods(r.projects())?.contains_key("validate.delta") {
        return Err("E_ROUTE_AMBIGUOUS: validate.delta".into());
    }
    Ok(())
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Receipt {
    pub plan: String,
    pub operation: String,
    pub method: String,
    pub process: ProcessResult,
    pub digest: String,
}
impl Receipt {
    pub fn new(plan: &Execution, op: &Operation, process: ProcessResult) -> Result<Self, String> {
        let mut receipt = Self {
            plan: plan.identity.clone(),
            operation: op.method.operation.clone(),
            method: wire::digest(&op.method)?,
            process,
            digest: String::new(),
        };
        receipt.digest = receipt.identity()?;
        Ok(receipt)
    }
    fn identity(&self) -> Result<String, String> {
        let mut v = serde_json::to_value(self).map_err(|e| e.to_string())?;
        v.as_object_mut().unwrap().remove("digest");
        wire::digest(&v)
    }
}
/// Compare actual producer observations, never populate missing observations from a plan.
pub fn compare(plan: &Execution, op: &Operation, receipt: Option<&Receipt>) -> Result<(), String> {
    plan.validate()?;
    let r = receipt.ok_or("E_ROUTE_MISSING: missing execution receipt")?;
    let tool = plan
        .tools
        .get(&op.method.tool)
        .ok_or("E_TOOL_BINDING: missing tool")?;
    let p = &r.process;
    let mut argv = vec![tool.path.to_str().ok_or("tool path")?.into()];
    argv.extend(op.method.argv.clone());
    if r.plan != plan.identity
        || r.operation != op.method.operation
        || r.method != wire::digest(&op.method)?
        || r.digest != r.identity()?
        || p.argv != argv
        || p.cwd != plan.root
        || p.executable != tool.path
        || p.sha256 != tool.sha256
        || p.environment != plan.environment
        || p.environment_digest != wire::digest(&plan.environment)?
        || p.stdin_sha256 != chrono_harness::sha256(&[])
        || p.stdout_sha256 != chrono_harness::sha256(&p.stdout_bytes)
        || p.stderr_sha256 != chrono_harness::sha256(&p.stderr_bytes)
        || p.stdout != String::from_utf8_lossy(&p.stdout_bytes)
        || p.stderr != String::from_utf8_lossy(&p.stderr_bytes)
    {
        return Err(
            "E_RECEIPT_MISMATCH: plan/method/tool/argv/cwd/input/environment/output".into(),
        );
    }
    Ok(())
}
pub fn execute_actions(r: &Registrations) -> BTreeMap<String, String> {
    r.node_data()
        .iter()
        .filter_map(|(id, node)| {
            if !id.starts_with("test:") {
                return None;
            }
            node.unique()
                .and_then(|d| d.value["actions"]["execute"]["operation"].as_str())
                .map(|op| (id.clone(), op.into()))
        })
        .collect()
}
pub fn judge(req: &Request) -> Response {
    let mut response = req.response(Status::Pass);
    let reader = facts::Reader::for_request(req);
    let result = reader
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|reader| evaluate(req, reader));
    if let Ok(reader) = &reader {
        reader.record(&mut response);
    }
    match result {
        Ok(outputs) => response.outputs.extend(outputs),
        Err(message) => {
            response.status = Status::Fail;
            response.findings.push(Finding {
                code: message.split(':').next().unwrap_or("E_ROUTES").into(),
                level: "error".into(),
                message,
                delta_refs: vec!["/request".into()],
                causes: vec![],
            });
        }
    }
    response
}
fn evaluate(req: &Request, reader: &facts::Reader) -> Result<BTreeMap<String, Value>, String> {
    req.validate()?;
    let (old, r, view) = chrono_judge_registration::views_with_reader(req, reader)?;
    canonical(req, &r)?;
    let inputs = chrono_judge_registration::inputs::validate(req, &old, &r)?;
    let impact: chrono_judge_filemap::Impact =
        serde_json::from_value(req.impact.clone()).map_err(|e| format!("E_IMPACT: {e}"))?;
    if impact.schema != chrono_judge_filemap::IMPACT_SCHEMA || impact.delta != req.delta {
        return Err("E_IMPACT: identity".into());
    }
    let mut selected: BTreeSet<_> = impact.tests.iter().cloned().collect();
    let replacements = chrono_judge_registration::replacements(&r)?;
    let requests = chrono_judge_registration::retirement_requests(&r)?;
    let deferred = chrono_judge_registration::downstream_validator(&r, &req.judge_id);
    for test in &impact.retired_tests {
        match requests.get(test) {
            Some(Some(replacement)) => {
                selected.insert(replacement.clone());
            }
            Some(None) if deferred => {}
            _ => return Err(format!("E_REQUIRED_TEST_REMOVED: {test}")),
        }
    }
    let candidate_plans = chrono_judge_registration::execution::plans(r.filemap())?;
    for required in &impact.required_tests {
        // Explicit method replacement or joint retirement is adjudicated by the
        // registered downstream validator; historical methods are retained data.
        if deferred && requests.contains_key(&required.node) && !required.candidate_present {
            continue;
        }
        let definitions = &impact.nodes[&required.node].base_definitions;
        if deferred
            && definitions.len() > 1
            && chrono_judge_registration::ambiguity_repaired(
                &r,
                &required.node,
                &definitions
                    .iter()
                    .map(|d| d.identity.clone())
                    .collect::<Vec<_>>(),
            )
        {
            continue;
        }

        let target = replacements.get(&required.node).unwrap_or(&required.node);
        if required.candidate_present
            && !impact.nodes[&required.node]
                .base_definitions
                .iter()
                .skip(1)
                .next()
                .is_some()
        {
            continue;
        }
        let plan = candidate_plans
            .get(target)
            .ok_or("E_REQUIRED_TEST_REMOVED: missing replacement plan")?;
        for definition in &impact.nodes[&required.node].base_definitions {
            if let Some(operation) = definition.value["actions"]["execute"]["operation"].as_str() {
                if !plan.operations.iter().any(|op| op == operation) {
                    return Err(format!(
                        "E_REQUIRED_TEST_REMOVED: replacement omits old execute obligation {operation}"
                    ));
                }
            }
        }
        if let Some(old_plan) = old
            .filemap()
            .get("execution_plans")
            .and_then(|p| p.get(&required.node))
        {
            for operation in old_plan["operations"].as_array().unwrap() {
                if !plan.operations.iter().any(|op| operation == op) {
                    return Err(
                        "E_REQUIRED_TEST_REMOVED: replacement omits historical prerequisite".into(),
                    );
                }
            }
        }
    }
    let env = serde_json::from_value(req.observations["environment"]["effective"].clone())
        .map_err(|e| e.to_string())?;
    let reused = chrono_judge_registration::reused_tools(&view)?;
    let plan = prepare(
        &req.candidate.root,
        binding(req, &inputs),
        &selected,
        &chrono_judge_registration::execution::plans(r.filemap())?,
        &chrono_judge_registration::execution::methods(r.projects())?,
        &execute_actions(&r),
        &r.config()["tools"],
        env,
        &reused,
    )?;
    // Every launched tool is an explicitly retained external input or a registered candidate file.
    for tool in plan.tools.values() {
        let registered = r.config()["environment"]["inputs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|i| {
                req.candidate.root.join(i["location"].as_str().unwrap()) == tool.path
                    && i["sha256"] == tool.sha256
            })
            || tool
                .path
                .strip_prefix(&req.candidate.root)
                .ok()
                .and_then(|p| p.to_str())
                .is_some_and(|p| {
                    r.filemap()["files"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|f| f["path"] == p)
                });
        if !registered {
            return Err(format!("E_INPUT_UNDECLARED: tool {}", tool.path.display()));
        }
    }
    Ok(BTreeMap::from([
        (
            "execution_plan".into(),
            serde_json::to_value(&plan).unwrap(),
        ),
        ("tools".into(), serde_json::to_value(&plan.tools).unwrap()),
        ("effective_inputs".into(), inputs),
        ("registration_view".into(), view),
        ("impact".into(), req.impact.clone()),
    ]))
}
