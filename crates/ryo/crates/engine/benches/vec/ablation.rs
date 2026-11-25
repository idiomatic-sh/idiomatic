//! Measurement only: runtime switches that turn a mechanism into a no-op (set ABL_<NAME>=1).
pub(crate) fn off(name: &'static str) -> bool {
    static SET: std::sync::OnceLock<std::collections::HashSet<String>> = std::sync::OnceLock::new();
    SET.get_or_init(|| {
        std::env::vars().filter(|(k, v)| k.starts_with("ABL_") && v == "1").map(|(k, _)| k[4..].to_owned()).collect()
    })
    .contains(name)
}
