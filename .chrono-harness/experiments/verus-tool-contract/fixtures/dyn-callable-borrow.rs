use vstd::prelude::*;

verus! {
    pub fn overlapping_borrows(observer: &mut dyn FnMut(u8) -> u8) {
        let first = &mut *observer;
        let second = &mut *observer;
        let pair = (first, second);
    }
}
