//! The typescript linter image: `vec_ts [--retain-findings] PACK REQUEST OUTPUT`.
#[path = "../allocator.rs"]
mod allocator;

use ryo_typescript as provider;

const DISTINCT_LOCATIONS: bool = false;

fn configured_provider(settings: Option<&str>, _pack: &std::path::Path) -> Result<ryo::languages::Language, String> {
    match settings {
        None => Ok(provider::linked()),
        Some(value) => Err(format!("unsupported provider settings {value}")),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ryo_lint_images::run(configured_provider, DISTINCT_LOCATIONS)
}
