#[rustfmt::skip]
#[deny(clippy:: /* why */ len_zero, dead_code)]
#[cfg_attr(test, rustfmt::skip(::a::r#b, c))]
pub fn f() {}
