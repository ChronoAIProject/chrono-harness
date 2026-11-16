use vstd::prelude::*;

static VALUE: u8 = 7;

macro_rules! static_from_rust_macro {
    () => {
        static MACRO_VALUE: u8 = {
            let mut value = 8u8;
            value += 1;
            value
        };
    };
}
static_from_rust_macro!();

pub fn nested_rust_static() -> u8 {
    static LOCAL: u8 = 10;
    LOCAL
}

verus! {
    exec static EXPLICIT: u8 ensures EXPLICIT == 11 { 11 }

    pub fn read_statics() {
        let a = &VALUE;
        let b = &VALUE;
        assert(a == b);
        let value = VALUE;
        assert(value <= 255);
        let from_macro = MACRO_VALUE;
        assert(from_macro <= 255);
        let explicit = EXPLICIT;
        assert(explicit == 11);
        let nested = nested_rust_static();
        assert(nested <= 255);
    }
}
