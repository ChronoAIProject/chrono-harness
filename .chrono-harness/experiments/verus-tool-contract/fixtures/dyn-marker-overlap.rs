use vstd::prelude::*;

verus! {
    trait ReadByte {}
    trait Specification {
        spec fn result() -> bool;
    }

    impl Specification for &dyn ReadByte {
        spec fn result() -> bool { true }
    }

    impl Specification for &(dyn ReadByte + Send) {
        spec fn result() -> bool { false }
    }
}
