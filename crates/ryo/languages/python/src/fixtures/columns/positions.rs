//! The column writer numbers syntax nodes first and tokens last in each segment.
//! The parse contract publishes preorder positions. Pair the children of the same
//! parser parent by ordinal, including tokens; byte ranges only verify that loan.
use ryo::languages::{self as l, Position, node_table};

pub(super) fn numbered(
    table: &l::NodeTable,
    public: &l::Elements,
) -> Result<Vec<Position>, String> {
    let count = node_table::element_count(table).0;
    let end = l::end_of(public).0 as usize;
    let unset = Position(u32::MAX);
    let mut positions = vec![unset; count];
    let mut stack = Vec::new();
    let mut map_segment = |root: Position,
                           published: Position,
                           positions: &mut [Position]|
     -> Result<(), String> {
        stack.push((root, published));
        while let Some((at, slot)) = stack.pop() {
            let n = at.0 as usize;
            if n >= count || slot.0 as usize >= end {
                return Err("node outside the numbered file".to_owned());
            }
            let span = node_table::span_of(table, at);
            if positions[n] != unset
                || node_table::kind_of(table, at) != l::kind_at(public, slot)
                || span.start != l::start_at(public, slot)
                || span.end != l::end_at(public, slot)
            {
                return Err(format!(
                    "syntax occurrence {} differs at published position {} (walk kind {:?}, {:?}; parse kind {:?}, {}..{})",
                    at.0,
                    slot.0,
                    node_table::kind_of(table, at),
                    span,
                    l::kind_at(public, slot),
                    l::start_at(public, slot).0,
                    l::end_at(public, slot).0
                ));
            }
            positions[n] = slot;
            let mut children = l::children(public, slot);
            for &child in node_table::children_of(table, at) {
                let paired = children
                    .next()
                    .ok_or_else(|| format!("missing child of published node {}", slot.0))?;
                stack.push((child, paired));
            }
            if children.next().is_some() {
                return Err(format!("extra child of published node {}", slot.0));
            }
        }
        Ok(())
    };
    if count != 0 && end != 0 {
        map_segment(Position(0), Position(0), &mut positions)?;
    }
    let published: std::collections::BTreeMap<_, _> = l::embedded_expressions(public)
        .map(|s| (s.host, s.root))
        .collect();
    for segment in node_table::embedded_segments(table) {
        let host = positions[segment.host.0 as usize];
        let root = published
            .get(&host)
            .ok_or_else(|| format!("no embedded parse for host {}", host.0))?;
        map_segment(segment.root, *root, &mut positions)?;
    }
    if count != end || positions.contains(&unset) {
        return Err(format!(
            "incomplete numbered parse: {count} walk elements, {end} published elements"
        ));
    }
    Ok(positions)
}

/// Rebind only node cells, preserving the qualified file and every scalar.
pub(crate) fn publish(
    columns: &mut [l::FactColumn],
    schema: &l::FileRelation,
    file: l::FileId,
    positions: &[Position],
) -> Result<(), l::ParseError> {
    for (column, (_, ty)) in columns.iter_mut().zip(schema.columns) {
        if *ty != l::CellKind::Node {
            continue;
        }
        for value in &mut column.0 {
            if *value == u64::MAX {
                continue;
            }
            let slot = positions
                .get((*value as u32) as usize)
                .filter(|_| *value >> 32 == u64::from(file.0))
                .ok_or_else(|| l::ParseError::Library {
                    reason: l::ToolMessage(format!("unbound node in {}", schema.name).into()),
                })?;
            *value = u64::from(file.0) << 32 | u64::from(slot.0);
        }
    }
    Ok(())
}
