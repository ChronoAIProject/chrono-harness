//! 错误对象完整状态的独立边界预期；同一驱动也作为 Rust 测试执行。
#![allow(unexpected_cfgs)]
#[cfg(verus_keep_ghost)]
#[path = "../../diagnostics/src/verified_error.rs"]
mod policy;
#[cfg(not(verus_keep_ghost))]
use chrono_diagnostics::{verified_error as policy, verus};
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::vpanic as panic;

verus! {

broadcast use vstd::string::group_string_axioms;

struct Node {
    code: u64,
    bytes: Vec<u8>,
}

struct Cause {
    code: u64,
    bytes: Vec<u8>,
}

fn expect(condition: bool)
    requires condition,
{
    if !condition { panic!(); }
}

fn node(code: u64) -> (out: Node)
    ensures out.code == code, out.bytes@ == seq![0u8, 255u8],
{
    Node { code, bytes: vec![0, 255] }
}

fn cause(code: u64) -> (out: Cause)
    ensures out.code == code, out.bytes@ == seq![255u8, 0u8],
{
    Cause { code, bytes: vec![255, 0] }
}

fn expect_node(node: &Node, code: u64)
    requires node.code == code, node.bytes@ == seq![0u8, 255u8],
{
    expect(node.code == code);
    expect(node.bytes.len() == 2);
    expect(node.bytes[0] == 0);
    expect(node.bytes[1] == 255);
}

fn expect_cause(cause: &Cause, code: u64)
    requires cause.code == code, cause.bytes@ == seq![255u8, 0u8],
{
    expect(cause.code == code);
    expect(cause.bytes.len() == 2);
    expect(cause.bytes[0] == 255);
    expect(cause.bytes[1] == 0);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn initialization_keeps_the_entire_node_and_has_no_causes() {
    let (node, source, related) = policy::initialize::<Node, Cause>(node(u64::MAX));
    expect_node(&node, u64::MAX);
    expect(source.is_none());
    expect(related.is_empty());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn setting_source_keeps_the_node_and_existing_related_causes() {
    let state = policy::initialize::<Node, Cause>(node(0));
    let state = policy::append_related(state, "cleanup".to_string(), cause(u64::MAX));
    let state = policy::set_source(state, cause(0));
    let (node, source, related) = policy::set_source(state, cause(u64::MAX));
    expect_node(&node, 0);
    match source {
        Some(source) => expect_cause(&source, u64::MAX),
        None => expect(false),
    }
    expect(related.len() == 1);
    expect(related[0].0 == "cleanup".to_string());
    expect_cause(&related[0].1, u64::MAX);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn append_keeps_source_duplicates_order_and_entire_error_values() {
    let state = policy::initialize::<Node, Cause>(node(u64::MAX));
    let state = policy::set_source(state, cause(0));
    let state = policy::append_related(state, "stderr".to_string(), cause(u64::MAX));
    let state = policy::append_related(state, "stderr".to_string(), cause(0));
    let (node, source, related) = policy::append_related(state, "".to_string(), cause(u64::MAX));
    expect_node(&node, u64::MAX);
    match source {
        Some(source) => expect_cause(&source, 0),
        None => expect(false),
    }
    expect(related.len() == 3);
    expect(related[0].0 == "stderr".to_string());
    expect(related[1].0 == "stderr".to_string());
    expect(related[2].0 == "".to_string());
    expect_cause(&related[0].1, u64::MAX);
    expect_cause(&related[1].1, 0);
    expect_cause(&related[2].1, u64::MAX);
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn new_error_node_keeps_each_text_field_and_the_entire_context() {
    let node = policy::ErrorNode::new(
        "owner::Native λ".to_string(),
        "E_INPUT\0IDENTITY".to_string(),
        "original failure\n终止".to_string(),
        vec![0u8, 255u8, 10u8],
    );
    expect(node.error_type == "owner::Native λ".to_string());
    expect(node.code == "E_INPUT\0IDENTITY".to_string());
    expect(node.message == "original failure\n终止".to_string());
    expect(node.context.len() == 3);
    expect(node.context[0] == 0);
    expect(node.context[1] == 255);
    expect(node.context[2] == 10);
    expect(node.source.is_none());
    expect(node.native_code.is_none());
    expect(node.traceback.is_none());
    expect(node.related.is_empty());
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn new_error_node_keeps_empty_text_and_context_without_inventing_a_cause() {
    let node = policy::ErrorNode::new(
        "".to_string(), "".to_string(), "".to_string(), Vec::<u8>::new(),
    );
    expect(node.error_type == "".to_string());
    expect(node.code == "".to_string());
    expect(node.message == "".to_string());
    expect(node.context.is_empty());
    expect(node.source.is_none());
    expect(node.native_code.is_none());
    expect(node.traceback.is_none());
    expect(node.related.is_empty());
}

pub open spec fn leaf_expected(node: &policy::ErrorNode<Vec<u8>>, marker: u8) -> bool {
    node.error_type@ == "Leaf\0λ"@
    && node.code@ == "E_LEAF"@
    && node.message@ == "leaf\nmessage"@
    && node.context@ == seq![0u8, marker, 255u8]
    && node.source.is_none()
    && node.native_code == Some(-7i64)
    && node.traceback.is_none()
    && node.related@.len() == 0
}

fn leaf(marker: u8) -> (out: policy::ErrorNode<Vec<u8>>)
    ensures leaf_expected(&out, marker),
{
    policy::ErrorNode {
        error_type: "Leaf\0λ".to_string(), code: "E_LEAF".to_string(),
        message: "leaf\nmessage".to_string(), context: vec![0, marker, 255],
        source: None, native_code: Some(-7), traceback: None, related: Vec::new(),
    }
}

fn expect_leaf(node: &policy::ErrorNode<Vec<u8>>, marker: u8)
    requires leaf_expected(node, marker),
{
    expect(node.error_type == "Leaf\0λ".to_string());
    expect(node.code == "E_LEAF".to_string());
    expect(node.message == "leaf\nmessage".to_string());
    expect(node.context.len() == 3);
    expect(node.context[0] == 0);
    expect(node.context[1] == marker);
    expect(node.context[2] == 255);
    expect(node.source.is_none());
    expect(node.native_code == Some(-7));
    expect(node.traceback.is_none());
    expect(node.related.is_empty());
}

pub open spec fn header_expected(node: &policy::ErrorNode<Vec<u8>>) -> bool {
    node.error_type@ == "Outer\0type"@
    && node.code@ == "E_OUTER"@
    && node.message@ == "message\nλ\0"@
    && node.context@ == seq![255u8, 0u8, 7u8, 0u8]
    && node.native_code == Some(i64::MIN)
    && node.traceback.is_some()
    && node.traceback.unwrap()@ == "栈\0\n"@
}

fn expect_header(node: &policy::ErrorNode<Vec<u8>>)
    requires header_expected(node),
{
    expect(node.error_type == "Outer\0type".to_string());
    expect(node.code == "E_OUTER".to_string());
    expect(node.message == "message\nλ\0".to_string());
    expect(node.context.len() == 4);
    expect(node.context[0] == 255);
    expect(node.context[1] == 0);
    expect(node.context[2] == 7);
    expect(node.context[3] == 0);
    expect(node.native_code == Some(i64::MIN));
    match &node.traceback {
        Some(trace) => expect(*trace == "栈\0\n".to_string()),
        None => expect(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn complete_assembly_keeps_all_fields_and_ordered_related_payloads() {
    let node = policy::ErrorNode::assemble(
        "Outer\0type".to_string(), "E_OUTER".to_string(), "message\nλ\0".to_string(),
        vec![255u8, 0u8, 7u8, 0u8], Some(Box::new(leaf(0))), Some(i64::MIN),
        Some("栈\0\n".to_string()),
        vec![
            policy::RelatedError::new("same-role".to_string(), leaf(255)),
            policy::RelatedError::new("same-role".to_string(), leaf(0)),
            policy::RelatedError::new("".to_string(), leaf(7)),
        ],
    );
    expect_header(&node);
    expect(node.related.len() == 3);
    expect(node.related[0].role == "same-role".to_string());
    expect(node.related[1].role == "same-role".to_string());
    expect(node.related[2].role == "".to_string());
    expect_leaf(&node.related[0].error, 255);
    expect_leaf(&node.related[1].error, 0);
    expect_leaf(&node.related[2].error, 7);
    match &node.source {
        Some(source) => expect_leaf(source, 0),
        None => expect(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn replacing_causes_preserves_metadata_and_can_clear_or_restore_causes() {
    let original = policy::ErrorNode {
        error_type: "Outer\0type".to_string(), code: "E_OUTER".to_string(),
        message: "message\nλ\0".to_string(), context: vec![255u8, 0u8, 7u8, 0u8],
        source: Some(Box::new(leaf(0))), native_code: Some(i64::MIN),
        traceback: Some("栈\0\n".to_string()),
        related: vec![policy::RelatedError { role: "old-role".to_string(), error: leaf(7) }],
    };
    let cleared = original.with_causes(None, Vec::new());
    expect_header(&cleared);
    expect(cleared.source.is_none());
    expect(cleared.related.is_empty());
    let restored = cleared.with_causes(
        Some(Box::new(leaf(255))),
        vec![policy::RelatedError { role: "\0λ".to_string(), error: leaf(0) }],
    );
    expect_header(&restored);
    expect(restored.related.len() == 1);
    expect(restored.related[0].role == "\0λ".to_string());
    expect_leaf(&restored.related[0].error, 0);
    match &restored.source {
        Some(source) => expect_leaf(source, 255),
        None => expect(false),
    }
}

#[cfg_attr(not(verus_keep_ghost), test)]
fn detaching_and_restoring_source_keeps_related_and_all_other_fields() {
    let original = policy::ErrorNode {
        error_type: "Outer\0type".to_string(), code: "E_OUTER".to_string(),
        message: "message\nλ\0".to_string(), context: vec![255u8, 0u8, 7u8, 0u8],
        source: Some(Box::new(leaf(255))), native_code: Some(i64::MIN),
        traceback: Some("栈\0\n".to_string()),
        related: vec![policy::RelatedError { role: "related\0λ".to_string(), error: leaf(0) }],
    };
    let (detached, source) = original.detach_source();
    expect_header(&detached);
    expect(detached.source.is_none());
    expect(detached.related.len() == 1);
    expect(detached.related[0].role == "related\0λ".to_string());
    expect_leaf(&detached.related[0].error, 0);
    match &source {
        Some(source) => expect_leaf(source, 255),
        None => expect(false),
    }
    let (detached_again, absent) = detached.detach_source();
    expect(absent.is_none());
    let restored = detached_again.with_source(source);
    expect_header(&restored);
    expect(restored.related.len() == 1);
    expect(restored.related[0].role == "related\0λ".to_string());
    expect_leaf(&restored.related[0].error, 0);
    match &restored.source {
        Some(source) => expect_leaf(source, 255),
        None => expect(false),
    }
}

}
