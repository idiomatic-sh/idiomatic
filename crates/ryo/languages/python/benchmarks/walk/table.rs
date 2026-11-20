//! One pre-order traversal of the parser's own tree: dense ids (also stored in the parser's node index so a typed child maps back
//! to its id in O(1)), a flat node table (kind, parent, subtree end, byte span, the in-place node), a per-kind bucket (CSR) and a
//! kinds-present bitset. Nothing else.

use ruff_python_ast::visitor::source_order::{SourceOrderVisitor, TraversalSignal};
use ruff_python_ast::{AnyNodeRef, HasNodeIndex as _, Mod, NodeIndex};
use ruff_text_size::Ranged as _;

pub(crate) const KINDS: usize = 128;

#[derive(Default)]
pub(crate) struct Table<'a> {
    pub kind: Vec<u8>,
    pub parent: Vec<u32>,
    pub end: Vec<u32>,
    pub start: Vec<u32>,
    pub stop: Vec<u32>,
    /// The parser's node, read in place.
    pub node: Vec<AnyNodeRef<'a>>,
    /// CSR of ids per kind: ids of kind k are `by_kind[offsets[k]..offsets[k+1]]`, in pre-order.
    pub offsets: Vec<u32>,
    pub by_kind: Vec<u32>,
    pub present: u128,
    stack: Vec<u32>,
}

impl<'a> Table<'a> {
    pub(crate) fn len(&self) -> usize {
        self.kind.len()
    }

    pub(crate) fn of_kind(&self, kind: u8) -> &[u32] {
        let k = kind as usize;
        &self.by_kind[self.offsets[k] as usize..self.offsets[k + 1] as usize]
    }

    /// Dense id of a node reached in place (a typed child), through the parser's node index.
    pub(crate) fn id_of(node: AnyNodeRef<'_>) -> u32 {
        node.node_index().load().as_u32().unwrap_or(u32::MAX)
    }

    pub(crate) fn build(&mut self, module: &'a Mod) {
        self.kind.clear();
        self.parent.clear();
        self.end.clear();
        self.start.clear();
        self.stop.clear();
        self.node.clear();
        self.stack.clear();
        self.present = 0;
        let root = AnyNodeRef::from(module);
        if self.enter_node(root).is_traverse() {
            match module {
                Mod::Module(m) => self.visit_body(&m.body),
                Mod::Expression(e) => self.visit_expr(&e.body),
            }
        }
        self.leave_node(root);
        // per-kind buckets by counting sort
        self.offsets.clear();
        self.offsets.resize(KINDS + 1, 0);
        for &k in &self.kind {
            self.offsets[k as usize + 1] += 1;
        }
        for k in 0..KINDS {
            self.offsets[k + 1] += self.offsets[k];
        }
        self.by_kind.clear();
        self.by_kind.resize(self.kind.len(), 0);
        let mut fill: [u32; KINDS] = [0; KINDS];
        fill.copy_from_slice(&self.offsets[..KINDS]);
        for (id, &k) in self.kind.iter().enumerate() {
            let slot = &mut fill[k as usize];
            self.by_kind[*slot as usize] = id as u32;
            *slot += 1;
        }
    }
}

impl<'a> SourceOrderVisitor<'a> for Table<'a> {
    fn enter_node(&mut self, node: AnyNodeRef<'a>) -> TraversalSignal {
        let id = self.kind.len() as u32;
        node.node_index().set(NodeIndex::from(id));
        let kind = node.kind() as u8;
        self.kind.push(kind);
        self.present |= 1u128 << (kind & 127);
        self.parent.push(self.stack.last().copied().unwrap_or(u32::MAX));
        self.end.push(0);
        let range = node.range();
        self.start.push(range.start().to_u32());
        self.stop.push(range.end().to_u32());
        self.node.push(node);
        self.stack.push(id);
        TraversalSignal::Traverse
    }

    /// The parser's walk enters no node for an interpolation's format spec; the provider has one (`joined-str`), so the table
    /// enters it too (bead ryo-ger6), between the interpolated expression and the spec's own elements.
    fn visit_interpolated_string_element(&mut self, element: &'a ruff_python_ast::InterpolatedStringElement) {
        let node = AnyNodeRef::from(element);
        match element {
            ruff_python_ast::InterpolatedStringElement::Interpolation(i) if i.format_spec.is_some() => {
                if self.enter_node(node).is_traverse() {
                    self.visit_expr(&i.expression);
                    let spec = i.format_spec.as_deref().expect("spec");
                    let spec_node = AnyNodeRef::from(spec);
                    self.enter_node(spec_node);
                    for e in spec.elements.iter() {
                        self.visit_interpolated_string_element(e);
                    }
                    self.leave_node(spec_node);
                }
                self.leave_node(node);
            }
            _ => ruff_python_ast::visitor::source_order::walk_interpolated_string_element(self, element),
        }
    }

    fn leave_node(&mut self, _node: AnyNodeRef<'a>) {
        if let Some(id) = self.stack.pop() {
            self.end[id as usize] = self.kind.len() as u32;
        }
    }
}
