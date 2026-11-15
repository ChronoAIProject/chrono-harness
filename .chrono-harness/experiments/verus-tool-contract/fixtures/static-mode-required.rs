use vstd::prelude::*;

verus! {
    // Verus 语法仍须显式声明静态变量的执行模式。
    static VALUE: u8 = 7;
}
