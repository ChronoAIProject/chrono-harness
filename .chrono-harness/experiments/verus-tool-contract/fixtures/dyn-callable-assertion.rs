use vstd::prelude::*;

verus! {
    trait ResultKind {
        spec fn byte_result() -> bool;
    }

    impl ResultKind for &dyn Fn(u8) -> u8 {
        open spec fn byte_result() -> bool { true }
    }

    impl ResultKind for &dyn Fn(u8) -> bool {
        open spec fn byte_result() -> bool { false }
    }

    pub fn unsupported_result(observer: &dyn Fn(u8) -> u8, value: u8)
        requires observer.requires((value,)),
    {
        let output = observer(value);
        assert(output == 7);
    }
}
