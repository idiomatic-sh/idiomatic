//! Normalize existing parser-owned payloads; never parse Python source spelling again.



use ryo::languages::{Flag, NameText};

/// Write the canonical decimal value from Ruff's parsed integer. Ruff retains
/// the radix-prefixed token as its payload when the integer exceeds u64.
pub(crate) fn integer_text(integer: &ruff_python_ast::Int, out: &mut String) -> Option<()> {
    use std::fmt::Write as _;
    out.clear();
    write!(out, "{integer}").ok()?;
    if integer.as_u64().is_some() { return Some(()); }
    out.retain(|ch| ch != '_');
    let radix = match out.as_bytes().get(..2) {
        Some(b"0x" | b"0X") => 16,
        Some(b"0o" | b"0O") => 8,
        Some(b"0b" | b"0B") => 2,
        _ => return (canonical_integer(NameText(out)) == Flag::Raised).then_some(()),
    };
    let value = num_bigint::BigUint::parse_bytes(&out.as_bytes()[2..], radix)?;
    out.clear();
    write!(out, "{value}").ok()
}

/// Validate a parser-normalized integer payload, without imposing a machine-width bound.
fn canonical_integer(value: NameText<'_>) -> Flag {
    let digits = value.0.strip_prefix('-').unwrap_or(value.0);
    if !digits.is_empty()
        && digits.bytes().all(|byte| byte.is_ascii_digit())
        && (digits.len() == 1 || !digits.starts_with('0'))
    {
        Flag::Raised
    } else {
        Flag::Lowered
    }
}
