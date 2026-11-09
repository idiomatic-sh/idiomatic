//! The library's own tree of a file, walked in the language's kinds: what the shape rail of the conformance holds our tree to. The
//! nodes are the library's abstract tree as it stands, each at its own range, the root at the whole file; every token of its stream,
//! and every stretch between two tokens the lexer gives no token for, is then put in the deepest node whose range holds it, a token of
//! no length at a node's edge being outside it. The adapter writes the table in one pass over the file; this places each piece by its
//! range alone, so a token the pass puts in the wrong node is a difference.

use std::collections::HashMap;

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{kind_bits, kind_name};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::source_order::{
    SourceOrderVisitor, TraversalSignal, walk_interpolated_string_element,
};
use ruff_python_ast::{AnyNodeRef, InterpolatedStringElement, NodeKind, PythonVersion};
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ruff_text_size::Ranged as _;
use ryo::languages::Offset;
use ryo::languages::contract::{Leafness, Walked};
use strum::IntoEnumIterator as _;

/// The library's nodes the walk names by hand: the ones whose name is not the standard's name with the library's prefix. Every
/// other node is its name without the prefix `Mod`, `Stmt`, `Expr` or `TypeParam`, in kebab case.
const NODE_NAMES: &[(&str, &str)] = &[
    ("ModExpression", "expression"),
    ("StmtExpr", "expr"),
    ("StmtIpyEscapeCommand", "ipy-escape-command"),
    ("ExprIpyEscapeCommand", "ipy-escape-command"),
    ("ExprNamed", "named-expr"),
    ("ExprIf", "if-exp"),
    ("ExprGenerator", "generator-exp"),
    ("ExprFString", "joined-str"),
    ("ExprTString", "template-str"),
    ("InterpolatedStringFormatSpec", "joined-str"),
    ("ExprStringLiteral", "constant"),
    ("ExprBytesLiteral", "constant"),
    ("ExprNumberLiteral", "constant"),
    ("ExprBooleanLiteral", "constant"),
    ("ExprNoneLiteral", "constant"),
    ("ExprEllipsisLiteral", "constant"),
    ("InterpolatedStringLiteralElement", "constant"),
    ("ExceptHandlerExceptHandler", "except-handler"),
    ("Parameters", "arguments"),
    ("Parameter", "arg"),
    ("WithItem", "withitem"),
    ("Arguments", "call-arguments"),
    ("TypeParams", "type-params"),
    ("PatternArguments", "pattern-arguments"),
    ("PatternKeyword", "pattern-keyword"),
    ("StringLiteral", "string-part"),
    ("BytesLiteral", "bytes-part"),
    ("FString", "f-string-part"),
    ("TString", "t-string-part"),
];

/// The library's tokens the walk names by hand: the ones whose name is not its name in lower case.
const TOKEN_NAMES: &[(&str, &str)] = &[
    ("Int", "number"),
    ("Float", "number"),
    ("Complex", "number"),
    ("FStringStart", "fstring-start"),
    ("FStringMiddle", "fstring-middle"),
    ("FStringEnd", "fstring-end"),
    ("TStringStart", "tstring-start"),
    ("TStringMiddle", "tstring-middle"),
    ("TStringEnd", "tstring-end"),
    ("IpyEscapeCommand", "errortoken"),
    ("Question", "errortoken"),
    ("Unknown", "errortoken"),
    ("NonLogicalNewline", "nl"),
    ("EndOfFile", "endmarker"),
    ("MinusEqual", "minequal"),
];

/// The kinds of the language by the name each has, the nodes and the extras apart from the tokens.
struct Names {
    /// The nodes and extras, by name.
    shapes: HashMap<&'static str, Kind>,
    /// The tokens, by name.
    tokens: HashMap<&'static str, Kind>,
}

/// The names the language gives its kinds.
fn names() -> Names {
    Names {
        shapes: Node::iter()
            .map(Kind::Node)
            .chain(Extra::iter().map(Kind::Extra))
            .map(|kind| (kind_name(kind).0, kind))
            .collect(),
        tokens: Token::iter()
            .map(Kind::Token)
            .map(|kind| (kind_name(kind).0, kind))
            .collect(),
    }
}

/// A word of the library in kebab case: `FunctionDef` is `function-def`.
fn kebab(word: &str) -> String {
    let mut out = String::new();
    for (place, letter) in word.chars().enumerate() {
        if letter.is_ascii_uppercase() && place > 0 {
            out.push('-');
        }
        out.push(letter.to_ascii_lowercase());
    }
    out
}

/// The name of the standard a node of the library stands for: by hand where it differs, else its own without its prefix.
fn node_name(kind: NodeKind, template: bool) -> String {
    let debug = format!("{kind:?}");
    if debug == "InterpolatedElement" {
        return if template {
            "interpolation"
        } else {
            "formatted-value"
        }
        .to_owned();
    }
    if let Some((_, name)) = NODE_NAMES.iter().find(|(own, _)| *own == debug) {
        return (*name).to_owned();
    }
    let bare = ["Mod", "Stmt", "Expr", "TypeParam"]
        .iter()
        .find_map(|prefix| debug.strip_prefix(prefix))
        .or_else(|| debug.strip_prefix("Pattern"))
        .unwrap_or(&debug);
    kebab(bare)
}

/// The name of the token kind of Python's `token` module a token of the library stands for: a keyword is a `NAME`.
fn token_name(kind: TokenKind) -> String {
    if kind == TokenKind::Identifier || kind.is_keyword() {
        return "name-token".to_owned();
    }
    let debug = format!("{kind:?}");
    match TOKEN_NAMES.iter().find(|(own, _)| *own == debug) {
        Some((_, name)) => (*name).to_owned(),
        None => debug.to_ascii_lowercase(),
    }
}

/// The nodes of the library's tree, gathered as the visitor enters them.
struct Gather {
    /// The nodes entered and not yet left, the innermost last.
    open: Vec<Walked>,
    /// The tree, once its root is left.
    root: Option<Walked>,
    /// Whether each open interpolated string is a template string, the innermost last.
    templates: Vec<bool>,
    /// The length of the file, in bytes.
    length: Offset,
    /// The language's kinds, by the names the library's nodes map to.
    names: Names,
}

impl Gather {
    /// The language's kind of a node of the library.
    fn kind_of_node(&self, node: NodeKind) -> Kind {
        let template = self.templates.last().copied().unwrap_or(false);
        self.names
            .shapes
            .get(node_name(node, template).as_str())
            .copied()
            .unwrap_or(Kind::Token(Token::Errortoken))
    }
}

impl<'a> SourceOrderVisitor<'a> for Gather {
    fn visit_interpolated_string_element(
        &mut self,
        interpolated_string_element: &'a InterpolatedStringElement,
    ) {
        // The library's own walk visits the parts of an interpolation's format spec and never the spec, which the standard has as
        // the `JoinedStr` of its `format_spec`: it is entered here as the node it is.
        let InterpolatedStringElement::Interpolation(interpolation) = interpolated_string_element
        else {
            walk_interpolated_string_element(self, interpolated_string_element);
            return;
        };
        let node = AnyNodeRef::from(interpolated_string_element);
        if self.enter_node(node).is_traverse() {
            self.visit_expr(&interpolation.expression);
            if let Some(spec) = &interpolation.format_spec {
                let spec_node = AnyNodeRef::from(spec.as_ref());
                if self.enter_node(spec_node).is_traverse() {
                    for part in &spec.elements {
                        self.visit_interpolated_string_element(part);
                    }
                }
                self.leave_node(spec_node);
            }
        }
        self.leave_node(node);
    }

    fn enter_node(&mut self, node: AnyNodeRef<'a>) -> TraversalSignal {
        let (start, end) = if self.open.is_empty() {
            (Offset(0), self.length)
        } else {
            (Offset(node.start().to_u32()), Offset(node.end().to_u32()))
        };
        // Ruff represents these pairs with one constructor and an explicit flag.
        // The language vocabulary preserves the distinct Python statement kinds.
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "only the three async statements share a ruff node with their sync form; every other node has its own kind"
        )]
        let kind = match node {
            AnyNodeRef::StmtFunctionDef(statement) if statement.is_async => {
                Kind::Node(Node::AsyncFunctionDef)
            }
            AnyNodeRef::StmtFor(statement) if statement.is_async => Kind::Node(Node::AsyncFor),
            AnyNodeRef::StmtWith(statement) if statement.is_async => Kind::Node(Node::AsyncWith),
            _ => self.kind_of_node(node.kind()),
        };
        self.open.push(Walked {
            kind: kind_bits(kind),
            leaf: Leafness::Node,
            start,
            end,
            children: Vec::new(),
        });
        if node.kind() == NodeKind::ExprTString {
            self.templates.push(true);
        } else if matches!(
            node.kind(),
            NodeKind::ExprFString | NodeKind::InterpolatedStringFormatSpec
        ) {
            self.templates.push(false);
        } else {
            // Any other node opens no string, and the innermost string stays the one already open.
        }
        TraversalSignal::Traverse
    }

    fn leave_node(&mut self, node: AnyNodeRef<'a>) {
        if matches!(
            node.kind(),
            NodeKind::ExprTString | NodeKind::ExprFString | NodeKind::InterpolatedStringFormatSpec
        ) {
            let _: Option<bool> = self.templates.pop();
        }
        if let Some(done) = self.open.pop() {
            match self.open.last_mut() {
                Some(parent) => parent.children.push(done),
                None => self.root = Some(done),
            }
        }
    }
}

/// Puts the token `piece` in the deepest node under `node` that holds it, among the children in the order of the file: the tokens
/// come in order, so it goes before the first node that starts where it ends or later.
fn place(node: &mut Walked, piece: Walked) {
    let inner = node.children.iter().position(|child| {
        // A piece of no length at the edge of a node is outside it.
        child.leaf == Leafness::Node
            && child.start <= piece.start
            && piece.end <= child.end
            && !(piece.start == piece.end
                && (piece.start == child.start || piece.start == child.end))
    });
    if let Some(at) = inner
        && let Some(child) = node.children.get_mut(at)
    {
        place(child, piece);
        return;
    }
    let at = node
        .children
        .iter()
        .position(|child| child.leaf == Leafness::Node && piece.end <= child.start)
        .unwrap_or(node.children.len());
    node.children.insert(at, piece);
}

/// A token of the walk.
fn token(kind: Kind, start: Offset, end: Offset) -> Walked {
    Walked {
        kind: kind_bits(kind),
        leaf: Leafness::Token,
        start,
        end,
        children: Vec::new(),
    }
}

/// The library's tree of `text`, with its tokens and the white space between them.
#[must_use]
pub(crate) fn walked(text: &str) -> Walked {
    walked_with(text, PythonVersion::latest())
}

/// Walk the reference with precisely the parser's selected Python 3 minor version.
pub(crate) fn walked_target<const MINOR: u8>(text: &str) -> Walked {
    walked_with(
        text,
        PythonVersion {
            major: 3,
            minor: MINOR,
        },
    )
}

/// Reference callback configured to the same target as the fixture parser.
fn walked_with(text: &str, version: PythonVersion) -> Walked {
    let options = ParseOptions::from(Mode::Module).with_target_version(version);
    let parsed = parse_unchecked(text, options);
    let length = Offset(u32::try_from(text.len()).unwrap_or(u32::MAX));
    let mut gather = Gather {
        open: Vec::new(),
        root: None,
        templates: Vec::new(),
        length,
        names: names(),
    };
    gather.visit_mod(parsed.syntax());
    let mut root = gather.root.unwrap_or_else(|| Walked {
        kind: kind_bits(Kind::Node(Node::Module)),
        leaf: Leafness::Node,
        start: Offset(0),
        end: length,
        children: Vec::new(),
    });
    let whitespace = Kind::Extra(Extra::Whitespace);
    let mut pieces = Vec::new();
    let mut end = Offset(0);
    for each in parsed.tokens() {
        let (start, stop) = (Offset(each.start().to_u32()), Offset(each.end().to_u32()));
        if end < start {
            pieces.push(token(whitespace, end, start));
        }
        let named = gather.names.tokens.get(token_name(each.kind()).as_str());
        pieces.push(token(
            named.copied().unwrap_or(Kind::Token(Token::Errortoken)),
            start,
            stop,
        ));
        end = end.max(stop);
    }
    if end < length {
        pieces.push(token(whitespace, end, length));
    }
    for piece in pieces {
        place(&mut root, piece);
    }
    root
}
