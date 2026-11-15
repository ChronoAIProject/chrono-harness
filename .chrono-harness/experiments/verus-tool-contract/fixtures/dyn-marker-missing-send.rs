use vstd::prelude::*;

verus! {
    trait ReadByte {
        fn read(&self) -> u8;
    }

    fn needs_send<T: ?Sized + Send>(value: &mut T) {}

    pub fn missing_send(value: &mut dyn ReadByte) {
        needs_send(value);
    }
}
