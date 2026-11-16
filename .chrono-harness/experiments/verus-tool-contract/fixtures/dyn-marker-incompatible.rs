use vstd::prelude::*;

verus! {
    trait ReadByte {
        spec fn value(&self) -> int;
        proof fn all_same(tracked &self)
            ensures forall|other: &Self| #[trigger] other.value() == self.value();
    }

    proof fn incompatible(tracked value: &(dyn ReadByte + Send)) {
        value.all_same();
    }
}
