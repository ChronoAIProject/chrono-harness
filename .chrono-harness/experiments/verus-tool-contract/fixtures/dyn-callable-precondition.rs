use vstd::prelude::*;

verus! {
    pub fn missing_contract(observer: &dyn Fn(u8) -> u8, value: u8) {
        let output = observer(value);
    }

    pub fn missing_mutable_contract(observer: &mut dyn FnMut(u8) -> u8, value: u8) {
        let output = observer(value);
    }
}
