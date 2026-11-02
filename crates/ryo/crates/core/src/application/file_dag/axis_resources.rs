//! Acquire and prepare immutable native-tree or indexed-table axis resources.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::{axis::PreparedAxis, views::BorrowedRows};
use crate::domain::table_cells::types::{RowIndex, TreeForm};
use crate::domain::table_model::{
    service::{cell, check_columns, index, push_row},
    types::{KeyIndex, On, Table},
};
use crate::domain::tree_walk::service::entity_of;
use ryo_language::domain::relations::{
    service::{node_file, node_position},
    types::Value,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Direction},
    plans::types::Operation,
};

pub(crate) fn prepare_axis<'a>(
    program: &crate::domain::operator::contract::resources::AxisResources,
    edges: BorrowedRows<'a>,
    domain: BorrowedRows<'a>,
) -> Result<PreparedAxis<'a>, OperatingError> {
    let crate::domain::operator::contract::resources::AxisResources {
        parent,
        child,
        domain_id,
        has_filter,
        domain_view,
        output,
        direction,
        ..
    } = program;
    if matches!(domain, BorrowedRows::Computed(_)) && domain.count() == 0 && output.is_empty() {
        return Ok(PreparedAxis::Empty);
    }
    if let BorrowedRows::Tree(tree) = edges {
        let domain_of_tree = matches!(domain,BorrowedRows::Tree(each) if each.form==TreeForm::Elements)
            && entity_of(domain_view, *domain_id) == ColumnIndex(0);
        if tree.form == TreeForm::Edges
            && (*parent, *child) == (ColumnIndex(0), ColumnIndex(1))
            && (!*has_filter && domain_view.is_empty() || domain_of_tree)
        {
            let among = if let BorrowedRows::Tree(each) = domain {
                (each.form == TreeForm::Elements).then_some(each)
            } else {
                None
            };
            let mut marks = Vec::new();
            if among.is_none() {
                marks.resize(tree.elements.kind.len(), false);
                let mut row = Vec::new();
                let entity = entity_of(domain_view, *domain_id);
                for at in 0..domain.count() {
                    let Some(row) = domain.read(RowIndex(at), &mut row) else {
                        continue;
                    };
                    if let Value::Node(node) = cell(row, entity)?
                        && node_file(node) == tree.file
                        && let Some(mark) = marks.get_mut(node_position(node).0 as usize)
                    {
                        *mark = true;
                    }
                }
            }
            return Ok(PreparedAxis::Native { tree, among, marks });
        }
    }
    if *has_filter
        || !domain_view.is_empty()
        || !output.is_empty()
        || matches!(
            direction,
            Direction::Deep | Direction::PreviousSibling | Direction::NextSibling
        )
    {
        return Err(OperatingError::WrongOperation);
    }
    check_columns(edges.kinds(), &[*parent, *child])?;
    check_columns(domain.kinds(), &[*domain_id])?;
    let (from, to) = match direction {
        Direction::Parent | Direction::Ancestor | Direction::NearestAncestor => ([*child], *parent),
        _ => ([*parent], *child),
    };
    let mut table = Table {
        kinds: edges.kinds().to_vec(),
        ..Table::default()
    };
    let mut buffer = Vec::new();
    for at in 0..edges.count() {
        if let Some(row) = edges.read(RowIndex(at), &mut buffer) {
            push_row(&mut table, row);
        }
    }
    let mut order = KeyIndex::default();
    index(&table, On::Columns(&from), &mut order)?;
    let mut members = hashbrown::HashSet::new();
    for at in 0..domain.count() {
        if let Some(row) = domain.read(RowIndex(at), &mut buffer) {
            let _new = members.insert(cell(row, *domain_id)?);
        }
    }
    Ok(PreparedAxis::Table {
        edges: table,
        index: order,
        from,
        to,
        domain: members,
    })
}
