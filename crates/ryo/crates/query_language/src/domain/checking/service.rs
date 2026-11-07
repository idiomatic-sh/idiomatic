//! The `checking` service: free functions named by verb over the checks of a module before its calls are expanded.

use super::types::{ImportAlias, ImportIndex};
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
                Vec::new(),
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
                Vec::new(),
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
                Vec::new(),
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
                Vec::new(),
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
                Vec::new(),
                format!("unknown relation cell type `{}`", column.kind.0),
            ));
        }
        if !names.insert(column.name.0.as_ref()) {
            return Err(located(
                source,
                column.span,
                Vec::new(),
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
                Vec::new(),
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
                    vec![trace_site(source, &target, import.span)],
                    format!("query module `{}` was not supplied in the bundle", target.0),
                ));
            }
        }
    }
    Ok(())
}

/// Rejects cyclic imports before expansion begins.
pub(crate) fn reject_cycles(
    root: &QuerySourceId,
    imports: &BTreeMap<QuerySourceId, ImportIndex>,
    fragments: &BTreeMap<QuerySourceId, FragmentProgram>,
) -> Result<(), LinkingError> {
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
                    trace,
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

    let mut colors = BTreeSet::new();
    let mut path = Vec::new();
    visit(root, imports, &mut colors, &mut path)?;
    for source in fragments.keys() {
        visit(source, imports, &mut colors, &mut path)?;
    }
    Ok(())
}
