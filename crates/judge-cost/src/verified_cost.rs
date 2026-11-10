//! Declared cost coordinate admission is a proved policy.
#![allow(unexpected_cfgs)]
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
#[path = "../../identifiers/src/lib.rs"]
pub mod identifiers;
#[cfg(not(verus_keep_ghost))]
pub use chrono_harness::verified_identifiers as identifiers;

verus! {

/// A cost declaration is unknown when any of its four coordinates is absent.
pub open spec fn unknown_coordinate(
    cpu_missing: bool,
    wall_missing: bool,
    peak_rss_missing: bool,
    io_missing: bool,
) -> bool {
    cpu_missing || wall_missing || peak_rss_missing || io_missing
}

/// The production judge uses this result to emit the warning and preserve the
/// exact missing coordinate list in its JSON adapter.
pub fn has_unknown_coordinate(
    cpu_missing: bool,
    wall_missing: bool,
    peak_rss_missing: bool,
    io_missing: bool,
) -> (unknown: bool)
    ensures unknown == unknown_coordinate(
        cpu_missing, wall_missing, peak_rss_missing, io_missing,
    ),
{
    cpu_missing || wall_missing || peak_rss_missing || io_missing
}

pub open spec fn coordinate_names(names: Seq<&str>) -> Seq<Seq<char>> {
    names.map(|i: int, name: &str| name@)
}

pub open spec fn missing_names(cpu: bool, wall: bool, peak: bool, io: bool) -> Seq<Seq<char>> {
    (if cpu { seq!["cpu_ms"@] } else { Seq::empty() })
        + (if wall { seq!["wall_ms"@] } else { Seq::empty() })
        + (if peak { seq!["peak_rss_bytes"@] } else { Seq::empty() })
        + (if io { seq!["io_bytes"@] } else { Seq::empty() })
}

/// 输出固定顺序的完整缺失坐标；已知值不加入，不排序或合并坐标。
pub fn missing_coordinate_names(cpu: bool, wall: bool, peak: bool, io: bool)
    -> (out: Vec<&'static str>)
    ensures coordinate_names(out@) == missing_names(cpu, wall, peak, io),
        out.len() == missing_names(cpu, wall, peak, io).len(),
        forall|i: int| 0 <= i < out.len() ==> (#[trigger] out@[i])@
            == missing_names(cpu, wall, peak, io)[i],
{
    let mut names = Vec::new();
    if cpu { names.push("cpu_ms"); }
    if wall { names.push("wall_ms"); }
    if peak { names.push("peak_rss_bytes"); }
    if io { names.push("io_bytes"); }
    assert(coordinate_names(names@) =~= missing_names(cpu, wall, peak, io));
    names
}

#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
pub struct PresenceError {
    pub message: &'static str,
    pub subject: String,
}

/// 两端都缺失时拒绝；仅基线存在时记入删除列表，其余保持原列表。
pub fn record_subject_presence(
    before: bool, after: bool, subject: String, removed: &mut Vec<String>,
) -> (out: Result<(), PresenceError>)
    ensures match out {
        Err(error) => !before && !after
            && error.message@ == "E_COST_REFERENCE: no declaration at either endpoint for "@
            && error.subject@ == subject@
            && removed@ == old(removed)@,
        Ok(()) => (before || after)
            && removed@ == if before && !after { old(removed)@.push(subject) } else { old(removed)@ },
    },
{
    if !before && !after {
        return Err(PresenceError {
            message: "E_COST_REFERENCE: no declaration at either endpoint for ", subject,
        });
    }
    if before && !after { removed.push(subject); }
    Ok(())
}

}
