//! 现役 CI 的执行结果转换、终态与离线结果集合合同。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
mod identifiers;
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verified_identifiers as identifiers;
use identifiers::equal_text;

#[cfg(verus_keep_ghost)]
#[path = "../../runner/src/verified_status.rs"]
pub mod result_types;
#[cfg(not(verus_keep_ghost))]
pub use chrono_harness::verified_status as result_types;
use result_types::{CheckResult, Status};

verus! {

pub open spec fn operation_status(state: Seq<char>) -> Status {
    if state == "passed"@ { Status::Passed }
    else if state == "blocked"@ { Status::Blocked }
    else { Status::Failed }
}

pub open spec fn operation_exit(state: Seq<char>, observed: Option<i32>) -> Option<i32> {
    match observed {
        Some(code) => if state == "passed"@ || code != 0 { Some(code) } else { None },
        None => None,
    }
}

pub open spec fn operation_cause(error: &Option<String>) -> Seq<char> {
    match error {
        Some(cause) => cause@,
        None => "registered command completed with matching receipt"@,
    }
}

/// 直接生成现役 CheckResult；失败回执中的零退出只留在原回执，不冒充失败退出。
pub fn operation_result(
    operation: &str, state: &str, error: &Option<String>, observed: Option<i32>,
) -> (row: CheckResult)
    ensures row.id@ == operation@,
        row.status == operation_status(state@),
        row.cause@ == operation_cause(error),
        row.exit_code == operation_exit(state@, observed),
{
    let passed = equal_text(state, "passed");
    let status = if passed { Status::Passed }
        else if equal_text(state, "blocked") { Status::Blocked }
        else { Status::Failed };
    let cause = match error {
        Some(cause) => cause.clone(),
        None => "registered command completed with matching receipt".to_string(),
    };
    let exit_code = match observed {
        Some(code) => if passed || code != 0 { Some(code) } else { None },
        None => None,
    };
    CheckResult { id: operation.to_string(), status, cause, exit_code }
}

pub open spec fn source_binding_failed(state: Seq<char>, binding: &Result<(), String>) -> bool {
    state == "passed"@ && binding.is_err()
}

/// 源码绑定失败不能沿用原 passed；原进程及其真实零退出仍保留在原回执。
pub fn operation_result_with_sources(
    operation: &str, state: &str, error: &Option<String>, observed: Option<i32>,
    binding: Result<(), String>,
) -> (row: CheckResult)
    ensures row.id@ == operation@,
        row.status == if source_binding_failed(state@, &binding) { Status::Failed }
            else { operation_status(state@) },
        row.cause@ == if source_binding_failed(state@, &binding) { binding.unwrap_err()@ }
            else { operation_cause(error) },
        row.exit_code == if source_binding_failed(state@, &binding) {
            operation_exit("failed"@, observed)
        } else { operation_exit(state@, observed) },
{
    match binding {
        Err(cause) => {
            if equal_text(state, "passed") {
                proof {
                    reveal_strlit("failed"); reveal_strlit("passed"); reveal_strlit("blocked");
                    assert("failed"@[0] != "passed"@[0]);
                    assert("failed"@.len() != "blocked"@.len());
                }
                operation_result(operation, "failed", &Some(cause), observed)
            } else { operation_result(operation, state, error, observed) }
        },
        Ok(()) => operation_result(operation, state, error, observed),
    }
}

pub open spec fn report_failure(rows: Seq<CheckResult>) -> bool {
    exists|i: int| 0 <= i < rows.len()
        && ((#[trigger] rows[i]).status == Status::Failed || rows[i].status == Status::Blocked)
}

pub open spec fn report_status(rows: Seq<CheckResult>) -> Status {
    if report_failure(rows) { Status::Failed } else { Status::Passed }
}

/// 本地／单元和离线汇总使用同一真实结果列表，失败或阻断均不能生成通过。
pub fn response_status(rows: &Vec<CheckResult>) -> (status: Status)
    ensures status == report_status(rows@),
{
    let mut i = 0;
    while i < rows.len()
        invariant 0 <= i <= rows.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] rows[j]).status != Status::Failed
                && rows[j].status != Status::Blocked,
        decreases rows.len() - i,
    {
        let row = &rows[i];
        if row.status.eq(&Status::Failed) || row.status.eq(&Status::Blocked) {
            assert(report_failure(rows@));
            return Status::Failed;
        }
        i += 1;
    }
    Status::Passed
}

/// 原异常产生唯一库存失败记录；状态与该实际记录同源。
pub fn inventory_failure(cause: String) -> (failure: (Status, Vec<CheckResult>))
    ensures failure.0 == report_status(failure.1@), failure.0 == Status::Failed,
        failure.1.len() == 1, failure.1[0].id@ == "ci.inventory"@,
        failure.1[0].status == Status::Failed, failure.1[0].cause@ == cause@,
        failure.1[0].exit_code.is_none(),
{
    let rows = vec![CheckResult {
        id: "ci.inventory".to_string(), status: Status::Failed, cause, exit_code: None,
    }];
    assert(report_failure(rows@));
    (Status::Failed, rows)
}

/// 诊断失败追加原有列表，保全先前顺序、重复及全部字段，并产生失败终态。
pub fn diagnostic_failure(rows: &mut Vec<CheckResult>) -> (status: Status)
    ensures rows.len() == old(rows).len() + 1,
        rows@.take(old(rows).len() as int) == old(rows)@,
        rows@.last().id@ == "ci.execution-diagnostics"@,
        rows@.last().status == Status::Failed,
        rows@.last().cause@ == "execution diagnostic publication failed; original errors retained"@,
        rows@.last().exit_code.is_none(),
        status == Status::Failed, status == report_status(rows@),
{
    rows.push(CheckResult {
        id: "ci.execution-diagnostics".to_string(), status: Status::Failed,
        cause: "execution diagnostic publication failed; original errors retained".to_string(),
        exit_code: None,
    });
    assert(report_failure(rows@));
    Status::Failed
}

pub open spec fn matching_result(
    values: Seq<CheckResult>, id: Seq<char>, status: Status, exit_code: Option<i32>,
) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i]).id@ == id
        && values[i].status == status && values[i].exit_code == exit_code
}

/// 同一条实际结果必须同时满足精确 ID、状态与退出码。
pub fn has_result(
    values: &Vec<CheckResult>, id: &str, status: &Status, exit_code: Option<i32>,
) -> (found: bool)
    ensures found <==> matching_result(values@, id@, *status, exit_code),
{
    let mut i = 0;
    while i < values.len()
        invariant 0 <= i <= values.len(),
            forall|j: int| 0 <= j < i ==> !(
                (#[trigger] values[j]).id@ == id@ && values[j].status == *status
                    && values[j].exit_code == exit_code),
        decreases values.len() - i,
    {
        let value = &values[i];
        if equal_text(value.id.as_str(), id) && value.status.eq(status)
            && value.exit_code == exit_code {
            return true;
        }
        i += 1;
    }
    false
}

pub open spec fn valid_result_states(values: Seq<CheckResult>, not_required: Seq<&str>) -> bool {
    matching_result(values, "ci.inventory"@, Status::Passed, None)
        && forall|i: int| 0 <= i < values.len()
            && text_member(not_required, (#[trigger] values[i]).id@)
                ==> values[i].status == Status::NotRequired
}

/// 库存标记要求 Passed/None；未要求结果只约束状态，保留原退出码合同。
pub fn matches_result_states(values: &Vec<CheckResult>, not_required: &Vec<&str>) -> (valid: bool)
    ensures valid <==> valid_result_states(values@, not_required@),
{
    if !has_result(values, "ci.inventory", &Status::Passed, None) { return false; }
    let mut i = 0;
    while i < values.len()
        invariant 0 <= i <= values.len(),
            matching_result(values@, "ci.inventory"@, Status::Passed, None),
            forall|j: int| 0 <= j < i
                && text_member(not_required@, (#[trigger] values[j]).id@)
                    ==> values[j].status == Status::NotRequired,
        decreases values.len() - i,
    {
        let value = &values[i];
        if contains_text(not_required, value.id.as_str())
            && !value.status.eq(&Status::NotRequired) { return false; }
        i += 1;
    }
    true
}

pub open spec fn text_member(values: Seq<&str>, target: Seq<char>) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i])@ == target
}

pub fn contains_text(values: &Vec<&str>, target: &str) -> (found: bool)
    ensures found <==> text_member(values@, target@),
{
    let mut i = 0;
    while i < values.len()
        invariant
            0 <= i <= values.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] values[j])@ != target@,
        decreases values.len() - i,
    {
        if equal_text(values[i], target) { return true; }
        i += 1;
    }
    false
}

pub open spec fn same_text_membership(actual: Seq<&str>, expected: Seq<&str>) -> bool {
    (forall|i: int| 0 <= i < actual.len() ==>
        text_member(expected, (#[trigger] actual[i])@))
        && (forall|i: int| 0 <= i < expected.len() ==>
            text_member(actual, (#[trigger] expected[i])@))
}

/// 不依赖顺序。重复 ID 的拒绝归响应协议，不由集合相等推导唯一性。
pub fn matches_text_membership(actual: &Vec<&str>, expected: &Vec<&str>) -> (matches: bool)
    ensures matches <==> same_text_membership(actual@, expected@),
{
    let mut i = 0;
    while i < actual.len()
        invariant 0 <= i <= actual.len(),
            forall|j: int| 0 <= j < i ==>
                text_member(expected@, (#[trigger] actual[j])@),
        decreases actual.len() - i,
    {
        if !contains_text(expected, actual[i]) { return false; }
        i += 1;
    }
    let mut i = 0;
    while i < expected.len()
        invariant 0 <= i <= expected.len(),
            forall|j: int| 0 <= j < actual.len() ==>
                text_member(expected@, (#[trigger] actual[j])@),
            forall|j: int| 0 <= j < i ==>
                text_member(actual@, (#[trigger] expected[j])@),
        decreases expected.len() - i,
    {
        if !contains_text(actual, expected[i]) { return false; }
        i += 1;
    }
    true
}

pub open spec fn text_values(values: Seq<&str>) -> Seq<Seq<char>> {
    values.map(|i: int, text: &str| text@)
}

pub open spec fn character_member(values: Seq<Seq<char>>, target: Seq<char>) -> bool {
    exists|i: int| 0 <= i < values.len() && (#[trigger] values[i]) == target
}

proof fn text_member_representation(values: Seq<&str>, target: Seq<char>)
    ensures text_member(values, target) <==> character_member(text_values(values), target),
{
    if text_member(values, target) {
        let i = choose|i: int| 0 <= i < values.len() && (#[trigger] values[i])@ == target;
        assert(text_values(values)[i] == target);
    }
    if character_member(text_values(values), target) {
        let i = choose|i: int| 0 <= i < text_values(values).len()
            && text_values(values)[i] == target;
        assert(values[i]@ == target);
    }
}

pub open spec fn result_membership(actual: Seq<&str>, expected: Seq<Seq<char>>) -> bool {
    (forall|i: int| 0 <= i < actual.len() ==>
        character_member(expected, (#[trigger] actual[i])@))
        && (forall|i: int| 0 <= i < expected.len() ==>
            text_member(actual, #[trigger] expected[i]))
}

proof fn result_membership_representation(actual: Seq<&str>, expected: Seq<&str>)
    ensures same_text_membership(actual, expected)
        <==> result_membership(actual, text_values(expected)),
{
    if same_text_membership(actual, expected) {
        assert forall|i: int| 0 <= i < actual.len() implies
            character_member(text_values(expected), (#[trigger] actual[i])@) by {
                text_member_representation(expected, actual[i]@);
            }
        assert forall|i: int| 0 <= i < text_values(expected).len() implies
            text_member(actual, #[trigger] text_values(expected)[i]) by {
                assert(text_values(expected)[i] == expected[i]@);
                assert(text_member(actual, expected[i]@));
            }
    }
    if result_membership(actual, text_values(expected)) {
        assert forall|i: int| 0 <= i < actual.len() implies
            text_member(expected, (#[trigger] actual[i])@) by {
                text_member_representation(expected, actual[i]@);
            }
        assert forall|i: int| 0 <= i < expected.len() implies
            text_member(actual, (#[trigger] expected[i])@) by {
                assert(text_values(expected)[i] == expected[i]@);
                assert(text_member(actual, text_values(expected)[i]));
            }
    }
}

pub open spec fn expected_result_ids(
    operations: Seq<&str>, not_required: Seq<&str>,
) -> Seq<Seq<char>> {
    seq!["ci.inventory"@] + text_values(operations) + text_values(not_required)
}

/// 借用登记列表，逐项保留原顺序、重复及完整字符串身份。
pub fn borrow_texts<'a>(values: &'a Vec<String>) -> (texts: Vec<&'a str>)
    ensures texts.len() == values.len(),
        forall|i: int| 0 <= i < values.len() ==> (#[trigger] texts[i])@ == values[i]@,
{
    let mut texts: Vec<&'a str> = Vec::new();
    let mut i = 0;
    while i < values.len()
        invariant 0 <= i <= values.len(), texts.len() == i,
            forall|j: int| 0 <= j < i ==> (#[trigger] texts[j])@ == values[j]@,
        decreases values.len() - i,
    {
        texts.push(values[i].as_str());
        i += 1;
    }
    texts
}

/// 借用真实报告字段，逐项保留原顺序、重复及完整字符串身份。
pub fn result_ids<'a>(values: &'a Vec<CheckResult>) -> (ids: Vec<&'a str>)
    ensures ids.len() == values.len(),
        forall|i: int| 0 <= i < values.len() ==> (#[trigger] ids[i])@ == values[i].id@,
{
    let mut ids: Vec<&'a str> = Vec::new();
    let mut i = 0;
    while i < values.len()
        invariant 0 <= i <= values.len(), ids.len() == i,
            forall|j: int| 0 <= j < i ==> (#[trigger] ids[j])@ == values[j].id@,
        decreases values.len() - i,
    {
        ids.push(values[i].id.as_str());
        i += 1;
    }
    ids
}

pub open spec fn report_id_member(actual: Seq<CheckResult>, id: Seq<char>) -> bool {
    exists|j: int| 0 <= j < actual.len() && (#[trigger] actual[j]).id@ == id
}

pub open spec fn report_result_membership(
    actual: Seq<CheckResult>, expected: Seq<Seq<char>>,
) -> bool {
    (forall|i: int| 0 <= i < actual.len() ==>
        character_member(expected, (#[trigger] actual[i]).id@))
        && (forall|i: int| 0 <= i < expected.len() ==>
            report_id_member(actual, #[trigger] expected[i]))
}

proof fn report_membership_representation(
    actual: Seq<CheckResult>, ids: Seq<&str>, expected: Seq<Seq<char>>,
)
    requires ids.len() == actual.len(),
        forall|i: int| 0 <= i < actual.len() ==> (#[trigger] ids[i])@ == actual[i].id@,
    ensures result_membership(ids, expected) <==> report_result_membership(actual, expected),
{
    if result_membership(ids, expected) {
        assert forall|i: int| 0 <= i < actual.len() implies
            character_member(expected, (#[trigger] actual[i]).id@) by {
                assert(actual[i].id@ == ids[i]@);
            }
        assert forall|i: int| 0 <= i < expected.len() implies
            report_id_member(actual, #[trigger] expected[i]) by {
                let j = choose|j: int| 0 <= j < ids.len()
                    && (#[trigger] ids[j])@ == expected[i];
                assert(actual[j].id@ == expected[i]);
            }
    }
    if report_result_membership(actual, expected) {
        assert forall|i: int| 0 <= i < ids.len() implies
            character_member(expected, (#[trigger] ids[i])@) by {
                assert(ids[i]@ == actual[i].id@);
            }
        assert forall|i: int| 0 <= i < expected.len() implies
            text_member(ids, #[trigger] expected[i]) by {
                let j = choose|j: int| 0 <= j < actual.len()
                    && (#[trigger] actual[j]).id@ == expected[i];
                assert(ids[j]@ == expected[i]);
            }
    }
}

/// 直接从真实报告取得 ID；调用方无需提供未经证明的字段投影。
pub fn matches_report_result_ids(
    actual: &Vec<CheckResult>, operations: &Vec<&str>, not_required: &Vec<&str>,
) -> (matches: bool)
    ensures matches <==> report_result_membership(
        actual@, expected_result_ids(operations@, not_required@)),
{
    let ids = result_ids(actual);
    let matches = matches_result_ids(&ids, operations, not_required);
    proof {
        report_membership_representation(
            actual@, ids@, expected_result_ids(operations@, not_required@));
    }
    matches
}

/// 期望结果恰为库存、登记操作和未要求测试；不从实际结果补期望。
pub fn matches_result_ids(
    actual: &Vec<&str>, operations: &Vec<&str>, not_required: &Vec<&str>,
) -> (matches: bool)
    ensures matches <==> result_membership(
        actual@, expected_result_ids(operations@, not_required@)),
{
    let mut expected = Vec::new();
    expected.push("ci.inventory");
    let mut i = 0;
    while i < operations.len()
        invariant 0 <= i <= operations.len(),
            text_values(expected@) == seq!["ci.inventory"@] + text_values(operations@).take(i as int),
        decreases operations.len() - i,
    {
        let ghost before = text_values(expected@);
        expected.push(operations[i]);
        proof {
            assert(text_values(expected@) =~= before.push(operations@[i as int]@));
            assert(text_values(operations@).take(i + 1)
                =~= text_values(operations@).take(i as int).push(operations@[i as int]@));
        }
        i += 1;
    }
    let mut i = 0;
    while i < not_required.len()
        invariant 0 <= i <= not_required.len(),
            text_values(expected@) == seq!["ci.inventory"@] + text_values(operations@)
                + text_values(not_required@).take(i as int),
        decreases not_required.len() - i,
    {
        let ghost before = text_values(expected@);
        expected.push(not_required[i]);
        proof {
            assert(text_values(expected@) =~= before.push(not_required@[i as int]@));
            assert(text_values(not_required@).take(i + 1)
                =~= text_values(not_required@).take(i as int).push(not_required@[i as int]@));
        }
        i += 1;
    }
    let matches = matches_text_membership(actual, &expected);
    proof {
        assert(text_values(expected@) =~= expected_result_ids(operations@, not_required@));
        result_membership_representation(actual@, expected@);
    }
    matches
}

}
