//! Every spelled kind of the language in real source: keywords in their positions, each operator, each literal form.

macro_rules! twice {
    ($x:expr, $($rest:tt)*) => { $x + $x };
    ($($y:ident),+ ; $z:ty) => { $( $y )* };
}

auto trait Marker {}

async fn waits() -> u8 {
    let value = async { 1u8 };
    value.await
}

const fn constant() -> bool {
    false
}

fn flows(items: &[i32], flag: bool) -> i32 {
    let mut total = 0;
    'outer: for item in items {
        while flag {
            if *item > 0 {
                continue 'outer;
            } else {
                break;
            }
        }
        match item {
            0 => return total,
            _ => total += item,
        }
    }
    total
}

fn closures() {
    let owned = String::new();
    let take = move || owned.len();
    let _ = take();
    let r#raw = 1;
    let reference = &r#raw;
    let _ = ref_of(reference);
}

fn ref_of(x: &i32) -> &i32 {
    match x {
        ref y => y,
    }
}

trait Shape {
    default fn area(&self) -> f64;
}

fn dynamic(shape: &dyn Shape) -> f64 {
    shape.area()
}

fn operators(a: i32, b: i32) -> bool {
    let mut c = a;
    c += b;
    c -= b;
    c *= b;
    c /= b;
    c %= b;
    c &= b;
    c |= b;
    c ^= b;
    c <<= 1;
    c >>= 1;
    let d = (a & b) | (a ^ b);
    let e = !a;
    let f = a << 1;
    let g = a >> 1;
    let h = a % b;
    let i = a / b;
    let j = -a;
    let k = a - b;
    let l = a * b;
    let m = a + b;
    let range = 0..10;
    let inclusive = 0..=10;
    let _ = (c, d, e, f, g, h, i, j, k, l, m, range, inclusive);
    a == b && a != b || a <= b && a >= b && a < b && a > b
}

fn question() -> Option<i32> {
    let x: Option<i32> = Some(1);
    let y = x?;
    Some(y)
}

fn literals() {
    let _c = 'a';
    let _b = b'a';
    let _bs = b"bytes";
    let _cs = c"c string";
    let _raw = r#"raw"#;
    let _n = 1_000i64;
    let _f = 1.5e3f32;
    let _t = true;
    let _u: () = ();
}

fn paths() {
    let _ = std::collections::HashMap::<i32, i32>::new();
    let _ = |x: i32| -> i32 { x };
    let _ = <i32 as Default>::default();
}

fn tilde_const<T: ~const Default>() {}

fn dollar() {
    let _ = stringify!($);
}

extern "C" {
    fn external(x: i32) -> i32;
}

union Either {
    a: u32,
    b: f32,
}

unsafe fn danger() {}

unsafe extern "C" {
    safe fn safe_call();
}

fn yields() {
    let _ = || {
        yield 1;
    };
    let _ = do_catch();
}

fn do_catch() -> i32 {
    try { 1 }
}

fn generic_asm() {
    unsafe {
        builtin # asm("nop", in(reg) 1, out(reg) _, inlateout(reg) 2 => _, lateout(reg) _, inout(reg) 3, options(pure, nomem, readonly, preserves_flags, noreturn, nostack, att_syntax, may_unwind, raw), clobber_abi("C"), sym danger, label {});
        builtin # format_args("{}", 1);
        builtin # offset_of(Either, a);
        builtin # deref(&1);
    }
}

fn unsized_types(_: &(dyn Shape + 'static)) {}

pub struct Wrap<T: ?Sized>(Box<T>);

fn patterns(value: Option<i32>) {
    if let Some(x) = value && x > 0 {
        let _ = x;
    }
    let (first, ..) = (1, 2, 3);
    let _ = first;
    let ptr = &raw const first;
    let _ = ptr;
}

fn labelled() {
    'a: loop {
        break 'a;
    }
}
