use vstd::prelude::*;

verus! {
    pub fn mismatched_output(observer: &dyn Fn(u8) -> u8) {
        let incompatible: &dyn Fn(u8) -> bool = observer;
    }
}
