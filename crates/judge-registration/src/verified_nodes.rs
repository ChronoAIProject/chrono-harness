//! 实际登记节点类型及其完整复制／相等语义。
#[cfg(not(verus_keep_ghost))]
use chrono_harness::verus;
#[cfg(verus_keep_ghost)]
use vstd::prelude::*;

verus! {

#[derive(Copy)]
#[cfg_attr(not(verus_keep_ghost), derive(Debug))]
pub enum NodeKind {
    File,
    Project,
    Script,
    Input,
    Test,
    Judge,
    Tool,
    Environment,
}

impl Clone for NodeKind {
    fn clone(&self) -> (out: Self)
        ensures out == *self,
    { *self }
}

#[cfg(verus_keep_ghost)]
impl vstd::std_specs::cmp::PartialEqSpecImpl for NodeKind {
    open spec fn obeys_eq_spec() -> bool { true }
    open spec fn eq_spec(&self, other: &Self) -> bool { *self == *other }
}

impl PartialEq for NodeKind {
    fn eq(&self, other: &Self) -> (equal: bool)
        ensures equal == (*self == *other),
    {
        match self {
            Self::File => matches!(other, Self::File),
            Self::Project => matches!(other, Self::Project),
            Self::Script => matches!(other, Self::Script),
            Self::Input => matches!(other, Self::Input),
            Self::Test => matches!(other, Self::Test),
            Self::Judge => matches!(other, Self::Judge),
            Self::Tool => matches!(other, Self::Tool),
            Self::Environment => matches!(other, Self::Environment),
        }
    }
}

impl Eq for NodeKind {}

}
