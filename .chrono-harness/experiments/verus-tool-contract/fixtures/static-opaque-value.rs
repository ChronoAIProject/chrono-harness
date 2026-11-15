use vstd::prelude::*;

static VALUE: u8 = 7;

verus! {
    // 没有 ensures 时，不从初始化表达式额外取得规格保证。
    fn invalid_assumption() {
        let value = VALUE;
        assert(value == 7);
    }
}
