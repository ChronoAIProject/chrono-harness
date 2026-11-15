use vstd::prelude::*;

verus! {
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
        requires old(observer).requires((value,)),
        ensures old(observer).ensures((value,), result),
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
}
