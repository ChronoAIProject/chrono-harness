use vstd::prelude::*;

verus! {
    pub trait ResultKind {
        spec fn byte_result() -> bool;
    }

    impl ResultKind for &dyn Fn(u8) -> u8 {
        open spec fn byte_result() -> bool { true }
    }

    impl ResultKind for &dyn Fn(u8) -> bool {
        open spec fn byte_result() -> bool { false }
    }

    proof fn distinct_output_types() {
        assert(<&dyn Fn(u8) -> u8 as ResultKind>::byte_result());
        assert(!<&dyn Fn(u8) -> bool as ResultKind>::byte_result());
    }

    pub fn invoke_shared(observer: &dyn Fn(u8) -> u8, value: u8) -> (result: u8)
        requires
            observer.requires((value,)),
            forall|output: u8| observer.ensures((value,), output) ==> output <= 10,
        ensures result <= 10,
    {
        observer(value)
    }

    pub fn invoke_mutable(
        observer: &mut dyn FnMut(u8) -> Result<u8, u8>,
        value: u8,
    ) -> (result: Result<u8, u8>)
        requires call_requires_unsized(&*old(observer), (value,)),
        ensures call_ensures_unsized(&*old(observer), (value,), result),
    {
        let result = observer(value);
        match result {
            Ok(output) => Ok(output),
            Err(error) => Err(error),
        }
    }

    pub fn coerce() {
        let mut observer = |value: u8| -> Result<u8, u8> { Err(value) };
        let dynamic: &mut dyn FnMut(u8) -> Result<u8, u8> = &mut observer;
    }

    pub fn invoke_report(
        observer: &mut dyn FnMut(&u8) -> Result<(), u8>,
        value: &u8,
    ) -> (result: Result<(), u8>)
        requires call_requires_unsized(&*old(observer), (value,)),
        ensures call_ensures_unsized(&*old(observer), (value,), result),
    {
        observer(value)
    }

    pub fn sized_callback<T: FnMut(u8) -> u8>(mut observer: T, value: u8) -> (result: u8)
        requires
            observer.requires((value,)),
            forall|output: u8| observer.ensures((value,), output) ==> output <= 10,
        ensures result <= 10,
    {
        observer(value)
    }
}
