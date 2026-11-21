//! The `checking` service: free functions named by verb over the checks of a module before its calls are expanded.

use super::types::{FragmentChecks, ImportAlias, ImportIndex};
use crate::domain::linking::errors::LinkingError;
use crate::domain::linking::service::located;
use crate::domain::linking::service::source_id;
use crate::domain::linking::service::trace_site;
use crate::domain::syntax::constants::fragment_scalar_kind_named;
use crate::domain::syntax::constants::relation_cell_kind_named;
use ryo_values::domain::program::types::FragmentProgram;
use ryo_values::domain::program::types::ImportDeclaration;
use ryo_values::domain::program::types::LetStep;
use ryo_values::domain::query_tokens::types::QueryBundle;
use ryo_values::domain::query_tokens::types::QuerySourceId;
use ryo_values::domain::query_tokens::types::QueryTraceSite;
use std::collections::BTreeMap;
use std::collections::BTreeSet;

/// Rejects repeated step names within one source module.
pub(crate) fn validate_step_names(
    source: &QuerySourceId,
    steps: &[LetStep],
) -> Result<(), LinkingError> {
    let mut names = BTreeSet::new();
    for step in steps {
        if !names.insert(step.name.0.as_ref()) {
            return Err(located(
                source,
                step.span,
                &[],
                format!(
                    "step `{}` is declared more than once in this source",
                    step.name.0
                ),
            ));
        }
    }
    Ok(())
}

/// Checks a fragment's input, parameter, and output signature for valid unique names.
pub(crate) fn validate_fragment_signature(
    source: &QuerySourceId,
    fragment: &FragmentProgram,
) -> Result<(), LinkingError> {
    let mut names = BTreeSet::new();
    for input in &fragment.inputs {
        if !names.insert(input.name.0.as_ref()) {
            return Err(located(
                source,
                input.span,
                &[],
                format!(
                    "fragment input `{}` is declared more than once",
                    input.name.0
                ),
            ));
        }
        validate_relation_signature(source, &input.relation)?;
    }
    for parameter in &fragment.parameters {
        if !names.insert(parameter.parameter.0.as_ref()) {
            return Err(located(
                source,
                parameter.span,
                &[],
                format!(
                    "fragment parameter `{}` is declared more than once",
                    parameter.parameter.0
                ),
            ));
        }
        if fragment_scalar_kind_named(&parameter.kind.0).is_none() {
            return Err(located(
                source,
                parameter.span,
                &[],
                format!("unsupported fragment parameter type `{}`", parameter.kind.0),
            ));
        }
    }
    validate_relation_signature(source, &fragment.output)
}

/// Checks that relation signature columns use supported kinds and unique names.
pub(crate) fn validate_relation_signature(
    source: &QuerySourceId,
    signature: &ryo_values::domain::program::types::RelationSignature,
) -> Result<(), LinkingError> {
    let mut names = BTreeSet::new();
    for column in &signature.columns {
        if relation_cell_kind_named(&column.kind.0).is_none() {
            return Err(located(
                source,
                column.span,
                &[],
                format!("unknown relation cell type `{}`", column.kind.0),
            ));
        }
        if !names.insert(column.name.0.as_ref()) {
            return Err(located(
                source,
                column.span,
                &[],
                format!(
                    "relation column `{}` is declared more than once",
                    column.name.0
                ),
            ));
        }
    }
    Ok(())
}

/// Indexes module imports by local alias and rejects duplicate aliases.
pub(crate) fn imports_for(
    source: &QuerySourceId,
    imports: &[ImportDeclaration],
) -> Result<ImportIndex, LinkingError> {
    let mut indexed = ImportIndex::default();
    for import in imports {
        if indexed
            .0
            .insert(ImportAlias(import.alias.0.clone()), import.clone())
            .is_some()
        {
            return Err(located(
                source,
                import.span,
                &[],
                super::constants::DUPLICATE_IMPORT_ALIAS,
            ));
        }
    }
    Ok(indexed)
}

/// Rejects imports whose target module is absent from the immutable bundle.
pub(crate) fn validate_import_targets(
    bundle: &QueryBundle,
    imports: &BTreeMap<QuerySourceId, ImportIndex>,
) -> Result<(), LinkingError> {
    for (source, aliases) in imports {
        for import in aliases.0.values() {
            let target = source_id(source, import)?;
            if !bundle.modules.contains_key(&target) {
                return Err(located(
                    source,
                    import.span,
                    &[trace_site(source, &target, import.span)],
                    format!("query module `{}` was not supplied in the bundle", target.0),
                ));
            }
        }
    }
    Ok(())
}

/// Walks the imports from `source`, refusing an import that reaches a module the walk is in.
fn visit(
    source: &QuerySourceId,
    imports: &BTreeMap<QuerySourceId, ImportIndex>,
    colors: &mut BTreeSet<QuerySourceId>,
    path: &mut Vec<QueryTraceSite>,
) -> Result<(), LinkingError> {
    let _new_color = colors.insert(source.clone());
    for import in imports
        .get(source)
        .into_iter()
        .flat_map(|index| index.0.values())
    {
        let target = source_id(source, import)?;
        let edge = trace_site(source, &target, import.span);
        if colors.contains(&target) {
            let mut trace = path.clone();
            trace.push(edge);
            return Err(located(
                source,
                import.span,
                &trace,
                format!("query module import cycle reaches `{}`", target.0),
            ));
        }
        path.push(edge);
        visit(&target, imports, colors, path)?;
        let _edge = path.pop();
    }
    let _removed_color = colors.remove(source);
    Ok(())
}

/// Rejects an import cycle a root reaches, before expansion begins: the walk from the root, over its own imports and then
/// the fragments'.
pub(crate) fn reject_root_cycles(
    root: &QuerySourceId,
    own: &ImportIndex,
    fragments: &BTreeMap<QuerySourceId, ImportIndex>,
) -> Result<(), LinkingError> {
    let mut colors = BTreeSet::from([root.clone()]);
    let mut path = Vec::new();
    for import in own.0.values() {
        let target = source_id(root, import)?;
        let edge = trace_site(root, &target, import.span);
        if colors.contains(&target) {
            return Err(located(
                root,
                import.span,
                &[edge],
                format!("query module import cycle reaches `{}`", target.0),
            ));
        }
        path.push(edge);
        visit(&target, fragments, &mut colors, &mut path)?;
        let _edge = path.pop();
    }
    Ok(())
}

/// The checks of a catalog's fragments, in the order a link makes them: each fragment's step names, signature and
/// aliases, stopping at the first refused; then each fragment's import targets against the bundle's modules; then the
/// import cycles among the fragments.
pub(crate) fn fragments_checked(
    bundle: &QueryBundle,
    fragments: &BTreeMap<QuerySourceId, FragmentProgram>,
) -> FragmentChecks {
    let mut checks = FragmentChecks::default();
    for (id, fragment) in fragments {
        let indexed = validate_step_names(id, &fragment.steps)
            .and_then(|()| validate_fragment_signature(id, fragment))
            .and_then(|()| imports_for(id, &fragment.imports));
        match indexed {
            Ok(indexed) => {
                let _first = checks.imports.insert(id.clone(), indexed);
            }
            Err(refused) => {
                checks.refused = Some(refused);
                return checks;
            }
        }
    }
    for (id, aliases) in &checks.imports {
        let one = BTreeMap::from([(id.clone(), aliases.clone())]);
        if let Err(refused) = validate_import_targets(bundle, &one) {
            checks.target = Some((id.clone(), refused));
            break;
        }
    }
    let (mut colors, mut path) = (BTreeSet::new(), Vec::new());
    for source in fragments.keys() {
        if let Err(cycle) = visit(source, &checks.imports, &mut colors, &mut path) {
            checks.cycle = Some(cycle);
            break;
        }
    }
    checks
}
