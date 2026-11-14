//! 嵌套证明只解析输入位置；根、输出、严格参数及其他选项不许改变。
use super::{InvocationKind, VerifierInvocation, identifiers::equal_text};
#[cfg(not(verus_keep_ghost))]
use verus_builtin_macros::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

pub open spec fn option_resolved(
    declared: Seq<String>, resolved: Seq<String>,
    before: &super::InvocationOption, after: &super::InvocationOption,
) -> bool {
    before.kind == after.kind && (
        declared[before.value as int]@ == resolved[after.value as int]@
        || before.kind == InvocationKind::Extern
            && super::invocation_identifier(vstd::utf8::encode_utf8(declared[before.value as int]@))
            && super::same_input_name(vstd::utf8::encode_utf8(declared[before.value as int]@),
                vstd::utf8::encode_utf8(resolved[after.value as int]@)))
}

fn resolved_option(
    declared: &Vec<String>, resolved: &Vec<String>,
    before: &super::InvocationOption, after: &super::InvocationOption,
) -> (same: bool)
    requires before.value < declared.len(), after.value < resolved.len(),
    ensures same == option_resolved(declared@, resolved@, before, after),
{
    if !super::same_invocation_kind(&before.kind, &after.kind) { return false; }
    if equal_text(declared[before.value].as_str(), resolved[after.value].as_str()) { return true; }
    match before.kind {
        InvocationKind::Extern =>
            super::valid_invocation_identifier(declared[before.value].as_str().as_bytes())
                && super::same_invocation_input_name(declared[before.value].as_str().as_bytes(),
                    resolved[after.value].as_str().as_bytes()),
        _ => false,
    }
}

pub open spec fn resolution_matches(
    declared: Seq<String>, resolved: Seq<String>,
    before: &VerifierInvocation, after: &VerifierInvocation,
) -> bool {
    declared[before.source as int]@ == resolved[after.source as int]@
        && declared[before.output as int]@ == resolved[after.output as int]@
        && before.options.len() <= after.options.len()
        && (forall|i: int| 0 <= i < before.options.len() ==>
            (#[trigger] before.options[i]).value < declared.len() && after.options[i].value < resolved.len()
                && option_resolved(declared, resolved, &before.options[i], &after.options[i]))
        && (forall|i: int| before.options.len() <= i < after.options.len() ==>
            (#[trigger] after.options[i]).kind == InvocationKind::Search)
}

pub open spec fn resolution_allowed(declared: Seq<String>, resolved: Seq<String>) -> bool {
    let before = super::interpreted_option_sequence(declared, 7, super::invocation_emit(declared));
    let after = super::interpreted_option_sequence(resolved, 7, super::invocation_emit(resolved));
    super::verifier_arguments(declared) && super::verifier_arguments(resolved)
        && declared[super::invocation_source(declared)]@ == resolved[super::invocation_source(resolved)]@
        && declared[super::invocation_output(declared)]@ == resolved[super::invocation_output(resolved)]@
        && before.len() <= after.len()
        && (forall|i: int| 0 <= i < before.len() ==>
            (#[trigger] before[i]).value < declared.len() && after[i].value < resolved.len()
                && option_resolved(declared, resolved, &before[i], &after[i]))
        && (forall|i: int| before.len() <= i < after.len() ==>
            (#[trigger] after[i]).kind == InvocationKind::Search)
}

/// 这里只核声明到调用的参数关系，不授予依赖、宏、VIR 或工具上下文资格。
pub fn validate_resolution(declared: &Vec<String>, resolved: &Vec<String>)
    -> (out: Result<(), String>)
    ensures out.is_ok() == resolution_allowed(declared@, resolved@),
        out.is_ok() ==> super::verifier_arguments(declared@)
        && super::verifier_arguments(resolved@)
        && exists|before: VerifierInvocation, after: VerifierInvocation|
            super::invocation_matches(&before, declared@)
                && super::invocation_matches(&after, resolved@)
                && resolution_matches(declared@, resolved@, &before, &after),
{
    let before = match super::parse_verifier_arguments(declared) {
        Some(value) => value,
        None => { return Err("E_VERUS_NESTED: noncanonical selected declaration".to_string()); },
    };
    let after = match super::parse_verifier_arguments(resolved) {
        Some(value) => value,
        None => { return Err("E_VERUS_NESTED: noncanonical resolved invocation".to_string()); },
    };
    if !equal_text(declared[before.source].as_str(), resolved[after.source].as_str())
        || !equal_text(declared[before.output].as_str(), resolved[after.output].as_str())
        || before.options.len() > after.options.len() {
        return Err("E_VERUS_NESTED: selected root, output or options differ".to_string());
    }
    let mut i = 0;
    while i < before.options.len()
        invariant 0 <= i <= before.options.len() <= after.options.len(),
            super::invocation_matches(&before, declared@), super::invocation_matches(&after, resolved@),
            forall|j: int| 0 <= j < i ==>
                (#[trigger] before.options[j]).value < declared.len() && after.options[j].value < resolved.len()
                    && option_resolved(declared@, resolved@, &before.options[j], &after.options[j]),
        decreases before.options.len() - i,
    {
        if before.options[i].value >= declared.len() || after.options[i].value >= resolved.len() {
            return Err("E_VERUS_NESTED: invalid option position".to_string());
        }
        if !resolved_option(declared, resolved, &before.options[i], &after.options[i]) {
            return Err("E_VERUS_NESTED: selected option or input name differs".to_string());
        }
        i += 1;
    }
    while i < after.options.len()
        invariant before.options.len() <= i <= after.options.len(),
            super::invocation_matches(&before, declared@), super::invocation_matches(&after, resolved@),
            forall|j: int| 0 <= j < before.options.len() ==>
                (#[trigger] before.options[j]).value < declared.len() && after.options[j].value < resolved.len()
                    && option_resolved(declared@, resolved@, &before.options[j], &after.options[j]),
            forall|j: int| before.options.len() <= j < i ==>
                (#[trigger] after.options[j]).kind == InvocationKind::Search,
        decreases after.options.len() - i,
    {
        if !matches!(after.options[i].kind, InvocationKind::Search) {
            return Err("E_VERUS_NESTED: undeclared option in resolved invocation".to_string());
        }
        i += 1;
    }
    assert(resolution_matches(declared@, resolved@, &before, &after));
    Ok(())
}

} // verus!
