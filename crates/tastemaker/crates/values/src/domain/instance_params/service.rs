//! The `instance_params` service: free functions named by verb that give the params of a spec their values at one of its
//! instances, with no IO and no ports.

use super::types::{Instance, InstanceParams};
use crate::domain::captures::types::{CaptureBinding, CaptureName, CaptureValue};
use crate::domain::entries::service::interpolate;
use crate::domain::entries::types::{ParamSource, ParamValue, Value};
use crate::domain::findings::constants::PARAMS;
use crate::domain::findings::service::{check_id, error};
use crate::domain::paths::service::name;
use crate::domain::scalars::types::TextValue;

/// The value of each of the spec's params at one instance, as text and typed; each problem is a `#params` finding.
#[must_use]
pub fn params(instance: Instance<'_>) -> InstanceParams {
    let Instance {
        identity,
        entry,
        spec,
        anchor,
        directory,
        parent,
        captures,
        workspace,
        written,
    } = instance;
    let mut found = InstanceParams::default();
    let pairs = captures
        .0
        .iter()
        .map(|(name, value)| CaptureBinding { name, value })
        .collect::<Vec<_>>();
    let rule = check_id(identity, PARAMS);
    for (param, value) in &entry.header.params.0 {
        if !spec.params.0.contains_key(param) {
            found.findings.push(error(
                rule.clone(),
                anchor.clone(),
                format!(
                    "`{identity}` passes `{}`, which its spec does not declare",
                    param.0
                ),
            ));
            continue;
        }
        let literal = match value {
            ParamValue::Literal(literal) => Some(literal.clone()),
            ParamValue::Param(reference) => parent.typed.get(reference).cloned(),
            ParamValue::Variable(_) => None,
        };
        if let Some(literal) = literal {
            let literal = interpolate(&literal, &pairs);
            drop(found.values.0.insert(
                CaptureName(param.0.clone()),
                CaptureValue(written.write(&literal).0.into()),
            ));
            drop(found.typed.insert(param.clone(), literal));
        } else {
            let reference = match value {
                ParamValue::Param(reference) => reference.0.to_string(),
                ParamValue::Literal(_) | ParamValue::Variable(_) => param.0.to_string(),
            };
            found.findings.push(error(
                rule.clone(),
                anchor.clone(),
                format!(
                    "`{identity}` passes `{{ param = \"{reference}\" }}`, but a tastemaker.toml entry has no params to pass"
                ),
            ));
        }
    }
    for (param, declaration) in &spec.params.0 {
        let key = CaptureName(param.0.clone());
        if found.values.0.contains_key(&key) {
            continue;
        }
        let captured = captures
            .0
            .get(&key)
            .cloned()
            .or_else(|| match declaration.source {
                ParamSource::DirectoryName => Some(CaptureValue(name(directory).0)),
                ParamSource::WorkspaceName => Some(CaptureValue(workspace.0.clone())),
                ParamSource::Given => None,
            });
        match captured {
            Some(captured) => {
                drop(
                    found
                        .typed
                        .insert(param.clone(), Value::Text(TextValue(captured.0.clone()))),
                );
                drop(found.values.0.insert(key, captured));
            }
            None => found.findings.push(error(
                rule.clone(),
                directory.clone(),
                format!(
                    "`{identity}` at `{}` has no value for its param `{}`",
                    directory.0, param.0
                ),
            )),
        }
    }
    found
}
