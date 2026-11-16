use vstd::prelude::*;

verus! {
    trait ReadByte {
        fn read(&self) -> (result: u8)
            ensures result <= 10;
    }

    impl ReadByte for u8 {
        fn read(&self) -> (result: u8) { *self / 32 }
    }

    fn needs_send<T: ?Sized + Send>(value: &mut T) {}
    fn needs_sync<T: ?Sized + Sync>(value: &T) {}
    fn needs_unpin<T: ?Sized + Unpin>(value: &T) {}

    fn forward(value: &mut (dyn ReadByte + Send + Sync + Unpin)) {
        needs_send(value);
        needs_sync(value);
        needs_unpin(value);
        let result = value.read();
        assert(result <= 10);
    }

    pub fn coerce() {
        let mut value = 100u8;
        let output: &mut (dyn ReadByte + Send + Sync + Unpin) = &mut value;
        forward(output);
    }
}
