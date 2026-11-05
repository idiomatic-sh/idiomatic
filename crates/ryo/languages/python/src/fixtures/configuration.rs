//! Source-declared targets of the pinned Python parser suite, confined to fixture setup.
use crate::outbound::syntax::fixtures::library;
use crate::settings::{PythonParserSettings, PythonTargetVersion};
use ryo::languages::contract::{ConfiguredFixture, Shape};
use serde::Deserialize;

/// Only target selection is part of this fixture contract; other upstream options stay unchanged.
#[derive(Deserialize)]
struct Options {
    /// A present declaration must be a string, and duplicate declarations are refused by serde.
    #[serde(default, rename = "target-version", deserialize_with = "target")]
    target: Option<String>,
}

/// Distinguish an absent target from an explicitly invalid null/non-string value.
fn target<'de, D>(input: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(input).map(Some)
}

/// Validate the leading fixture metadata before creating a parser or calling either reference.
fn declared_target(text: &str) -> Result<Option<String>, String> {
    let mut declared = None;
    for line in text.lines() {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            break;
        }
        if !line.starts_with("# parse_options") {
            continue;
        }
        if declared.is_some() {
            return Err("duplicate parse_options declaration".to_owned());
        }
        let json = line
            .strip_prefix("# parse_options:")
            .ok_or_else(|| "malformed parse_options declaration".to_owned())?;
        if !json.trim_start().starts_with('{') {
            return Err("parse_options must be a JSON object".to_owned());
        }
        let options: Options = serde_json::from_str(json)
            .map_err(|error| format!("malformed parse_options JSON: {error}"))?;
        declared = Some(options.target);
    }
    Ok(declared.flatten())
}

/// Construct matching parser, shape, and row callbacks using a validated target.
fn configured<const MINOR: u8>(target: PythonTargetVersion) -> ConfiguredFixture {
    ConfiguredFixture {
        language: crate::linked_with_settings(PythonParserSettings::new(target)),
        shape: Shape::Walk(library::walk_for::<MINOR>()),
        oracle: library::oracle_for::<MINOR>(),
    }
}

/// Choose an explicit immutable fixture target; an absent declaration retains the caller's default.
///
/// # Errors
/// Returns the malformed declaration or unsupported target, never silently using Latest.
pub fn select(text: &str) -> Result<Option<ConfiguredFixture>, String> {
    let Some(target) = declared_target(text)? else {
        return Ok(None);
    };
    let selected = match target.as_str() {
        "3.7" => configured::<7>(PythonTargetVersion::Py37),
        "3.8" => configured::<8>(PythonTargetVersion::Py38),
        "3.9" => configured::<9>(PythonTargetVersion::Py39),
        "3.10" => configured::<10>(PythonTargetVersion::Py310),
        "3.11" => configured::<11>(PythonTargetVersion::Py311),
        "3.12" => configured::<12>(PythonTargetVersion::Py312),
        "3.13" => configured::<13>(PythonTargetVersion::Py313),
        "3.14" => configured::<14>(PythonTargetVersion::Py314),
        "3.15" => configured::<15>(PythonTargetVersion::Py315),
        _ => return Err(format!("unsupported fixture target-version {target:?}")),
    };
    Ok(Some(selected))
}

#[cfg(test)]
#[path = "configuration_test.rs"]
mod tests;
