use vstd::prelude::*;

verus! {
    trait ReadByte {
        fn read(&self) -> (result: u8)
            ensures result <= 10;
    }

    pub fn unsupported_value(value: &(dyn ReadByte + Send)) {
        let result = value.read();
        assert(result == 7);
    }
}
