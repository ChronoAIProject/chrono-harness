use vstd::prelude::*;

verus! {
    const fn under_ten(value: u8) -> u8
        requires value < 10,
    { value }
}

// 预期由初始化调用的真实前置条件拒绝，不能只拒绝 static 语法。
static INVALID: u8 = under_ten(10);
