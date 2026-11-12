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

}
