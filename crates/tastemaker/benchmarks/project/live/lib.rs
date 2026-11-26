//! The Tastemaker benchmark package: the bench `tm` (the product's main with the bench entry points in front of it) and its capture
//! replay. The linter itself is the workspace SDK's `Linter`; nothing here patches the product.
mod capture;
mod linked;

pub use capture::{capture, capture_one, capture_retained, capture_then, exit_process};
pub use linked::Linked;

/// The workspace's nine languages, as the bench entry points read them (the same list the product links).
#[must_use]
pub fn languages() -> ryo_sdk::linter::Languages {
    ryo_sdk::linter::Languages(vec![
        ryo_css::linked(),
        ryo_go::linked(),
        ryo_html::linked(),
        ryo_json::linked(),
        ryo_markdown::linked(),
        ryo_python::linked(),
        ryo_rust::linked(),
        ryo_toml::linked(),
        ryo_typescript::linked(),
    ])
}
