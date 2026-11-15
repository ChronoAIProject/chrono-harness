use vstd::prelude::*;

static VALUE: u8 = 7;

verus! {
    // 普通 Rust static 不能成为规格态值。
    proof fn invalid_read() {
        let value = VALUE;
    }
}
