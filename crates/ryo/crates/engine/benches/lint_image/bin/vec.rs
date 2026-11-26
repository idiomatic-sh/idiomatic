//! The python linter image: `vec [--retain-findings] PACK REQUEST OUTPUT`.
#[path = "../allocator.rs"]
mod allocator;

use ryo_python as provider;

const DISTINCT_LOCATIONS: bool = true;

fn configured_provider(settings: Option<&str>, _pack: &std::path::Path) -> Result<ryo::languages::Language, String> {
    use provider::settings::{PythonParserSettings, PythonTargetVersion};
    match settings {
        None => Ok(provider::linked()),
        Some("python-target=3.10") => Ok(provider::linked_with_settings(PythonParserSettings::new(PythonTargetVersion::Py310))),
        Some(value) => Err(format!("unsupported provider settings {value}")),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    ryo_lint_images::run(configured_provider, DISTINCT_LOCATIONS)
}
