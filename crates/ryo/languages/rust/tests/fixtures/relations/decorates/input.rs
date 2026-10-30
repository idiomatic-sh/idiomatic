#[inline]
fn run() {}

#[derive(Debug, Clone)]
#[serde(rename = "x")]
struct S {
    #[allow(dead_code)]
    field: u8,
}

#[path = "inner.rs"]
mod inner;

#[cfg(test)]
fn check() {}
