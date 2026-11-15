use vstd::prelude::*;

verus! {
    pub fn missing_contract(observer: &dyn Fn(u8) -> u8, value: u8) {
        let output = observer(value);
    }
}
