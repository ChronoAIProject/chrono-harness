use vstd::prelude::*;

verus! {
    trait ReadByte {
        fn read(&self) -> u8;
    }

    fn use_both(first: &mut (dyn ReadByte + Send), second: &mut (dyn ReadByte + Send)) {}

    pub fn overlapping_borrows(value: &mut (dyn ReadByte + Send)) {
        use_both(value, value);
    }
}
