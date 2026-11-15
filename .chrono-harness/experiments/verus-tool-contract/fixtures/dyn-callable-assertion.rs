use vstd::prelude::*;

verus! {
    pub fn unsupported_result(observer: &dyn Fn(u8) -> u8, value: u8)
        requires observer.requires((value,)),
    {
        let output = observer(value);
        assert(output == 7);
    }
}
