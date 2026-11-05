//! What the parser gives, which the adapter does not control, and its conversion into the language's words: the one declaration of what
//! each of the library's kinds, fields and spellings is in Go, and in `go/ast` and `go/token`. Every named kind of the grammar is
//! mapped to a form, or is a kind the adapter does not know; every anonymous kind to the token of `go/token` that spells it. The tests
//! hold both to the grammar's own list of kinds, so a kind the grammar gains is a failing test until it is mapped.

use tree_sitter::Node as Library;

use super::types::{Misreading, Unrecognized, Unspelled};
use crate::domain::grammar::types::{Form, Role};
use crate::domain::nodes::types::GoNode;
use crate::domain::tokens::types::Token;

/// A named kind of the library's tree.
pub(super) struct KindSchema<'a>(pub &'a str);

/// The name of a field of the library's grammar.
pub(super) struct FieldSchema<'a>(pub &'a str);

impl<'a> TryFrom<KindSchema<'a>> for Form {
    type Error = Unrecognized<'a>;

    /// What the named kind of the library is in Go; a kind the adapter does not know is refused, named.
    fn try_from(schema: KindSchema<'a>) -> Result<Self, Self::Error> {
        Ok(match schema.0 {
            "source_file" => Self::Source,
            "ERROR" => Self::Broken,
            "comment" => Self::Comment,
            "type_identifier" => Self::TypeName,
            "identifier" | "field_identifier" | "package_identifier" | "label_name"
            | "blank_identifier" | "nil" | "true" | "false" | "iota" => Self::Name,
            "dot" => Self::Dot,
            "int_literal" => Self::Literal(Token::Int),
            "float_literal" => Self::Literal(Token::Float),
            "imaginary_literal" => Self::Literal(Token::Imag),
            "rune_literal" => Self::Literal(Token::Char),
            "raw_string_literal" | "interpreted_string_literal" => Self::Text,
            "escape_sequence"
            | "interpreted_string_literal_content"
            | "raw_string_literal_content" => Self::Inside,
            "statement_list" => Self::StatementList,
            "range_clause" => Self::RangeClause,
            "expression_list" | "import_spec_list" | "var_spec_list" | "argument_list"
            | "variadic_argument" | "literal_element" | "for_clause" | "package_clause"
            | "type_arguments" | "field_name_list" => Self::Transparent,
            "import_spec" => Self::Node(GoNode::ImportSpec),
            "const_spec" | "var_spec" => Self::Node(GoNode::ValueSpec),
            "type_spec" | "type_alias" => Self::Node(GoNode::TypeSpec),
            "parenthesized_type" | "parenthesized_expression" => Self::Node(GoNode::ParenExpr),
            "pointer_type" => Self::Node(GoNode::StarExpr),
            "array_type" | "slice_type" => Self::Node(GoNode::ArrayType),
            "map_type" => Self::Node(GoNode::MapType),
            "channel_type" => Self::Node(GoNode::ChanType),
            "negated_type" => Self::Node(GoNode::UnaryExpr),
            "qualified_type" | "selector_expression" => Self::Node(GoNode::SelectorExpr),
            "parameter_list" => Self::Parameters,
            "type_parameter_list" | "field_declaration_list" => Self::Node(GoNode::FieldList),
            "parameter_declaration" | "type_parameter_declaration" => Self::Node(GoNode::Field),
            "block" => Self::Node(GoNode::BlockStmt),
            "expression_statement" => Self::Node(GoNode::ExprStmt),
            "send_statement" => Self::Node(GoNode::SendStmt),
            "inc_statement" | "dec_statement" => Self::Node(GoNode::IncDecStmt),
            "assignment_statement" | "short_var_declaration" => Self::Node(GoNode::AssignStmt),
            "return_statement" => Self::Node(GoNode::ReturnStmt),
            "go_statement" => Self::Node(GoNode::GoStmt),
            "defer_statement" => Self::Node(GoNode::DeferStmt),
            "if_statement" => Self::Node(GoNode::IfStmt),
            "empty_statement" => Self::Node(GoNode::EmptyStmt),
            "expression_case" | "default_case" | "type_case" => Self::Node(GoNode::CaseClause),
            "communication_case" => Self::Node(GoNode::CommClause),
            "binary_expression" => Self::Node(GoNode::BinaryExpr),
            "index_expression" => Self::Node(GoNode::IndexExpr),
            "slice_expression" => Self::Node(GoNode::SliceExpr),
            "type_assertion_expression" => Self::Node(GoNode::TypeAssertExpr),
            "composite_literal" => Self::Node(GoNode::CompositeLit),
            "keyed_element" => Self::Node(GoNode::KeyValueExpr),
            "struct_type" => Self::Node(GoNode::StructType),
            "function_declaration" => Self::Function,
            "method_declaration" => Self::Method,
            "func_literal" => Self::Closure,
            "function_type" => Self::Signature,
            "method_elem" => Self::MethodElement,
            "type_elem" | "type_constraint" => Self::Union,
            "generic_type" => Self::GenericType,
            "type_instantiation_expression" => Self::Instantiation,
            "call_expression" => Self::Call,
            "type_conversion_expression" => Self::Conversion,
            "unary_expression" => Self::Unary,
            "interface_type" => Self::Interface,
            "field_declaration" => Self::FieldDeclaration,
            "variadic_parameter_declaration" => Self::Variadic,
            "implicit_length_array_type" => Self::ImplicitArray,
            "const_declaration" | "var_declaration" | "type_declaration" => Self::Declaration,
            "import_declaration" => Self::Import,
            "expression_switch_statement" => Self::Switch,
            "type_switch_statement" => Self::TypeSwitch,
            "select_statement" => Self::Select,
            "for_statement" => Self::Loop,
            "receive_statement" => Self::Receive,
            "literal_value" => Self::Elements,
            "break_statement" => Self::Branch(Token::Break),
            "continue_statement" => Self::Branch(Token::Continue),
            "goto_statement" => Self::Branch(Token::Goto),
            "fallthrough_statement" => Self::Branch(Token::Fallthrough),
            "labeled_statement" => Self::Labeled,
            unseen => return Err(Unrecognized(unseen)),
        })
    }
}

impl From<FieldSchema<'_>> for Option<Role> {
    /// The part the library's field of that name is; none for a field the language reads nothing from.
    fn from(schema: FieldSchema<'_>) -> Self {
        match schema.0 {
            "name" => Some(Role::Name),
            "type" => Some(Role::Type),
            "result" => Some(Role::Result),
            "parameters" => Some(Role::Parameters),
            "type_arguments" => Some(Role::TypeArguments),
            "value" => Some(Role::Value),
            "alias" => Some(Role::Alias),
            "left" => Some(Role::Left),
            _other => None,
        }
    }
}

/// The form of the anonymous node the library names `text`: the token that spells it, or a spelling the language has no token for.
pub(super) fn spelling(text: &str) -> Form {
    Token::try_from(text).map_or(Form::Unspelled, Form::Spelled)
}

/// The form of a node of the library: what a named kind is in Go, or what an anonymous one spells.
pub(super) fn form_of(node: &Library<'_>) -> Form {
    if node.is_named() {
        Form::try_from(KindSchema(node.kind())).unwrap_or(Form::Unknown)
    } else {
        spelling(node.kind())
    }
}

/// The part the node plays in its parent, by the name of the field the parent holds it in.
pub(super) fn role_of(field: Option<&str>) -> Option<Role> {
    let name = field?;
    Option::<Role>::from(FieldSchema(name))
}

impl<'a> TryFrom<&'a str> for Token {
    type Error = Unspelled<'a>;

    /// The token the library writes `spelling` for: an operator, a delimiter or a keyword. The library names its anonymous nodes by
    /// their text, which is open, so a spelling the language has no token for is refused, named.
    fn try_from(spelling: &'a str) -> Result<Self, Self::Error> {
        Ok(match spelling {
            "+" => Token::Add,
            "-" => Token::Sub,
            "*" => Token::Mul,
            "/" => Token::Quo,
            "%" => Token::Rem,
            "&" => Token::And,
            "|" => Token::Or,
            "^" => Token::Xor,
            "<<" => Token::Shl,
            ">>" => Token::Shr,
            "&^" => Token::AndNot,
            "+=" => Token::AddAssign,
            "-=" => Token::SubAssign,
            "*=" => Token::MulAssign,
            "/=" => Token::QuoAssign,
            "%=" => Token::RemAssign,
            "&=" => Token::AndAssign,
            "|=" => Token::OrAssign,
            "^=" => Token::XorAssign,
            "<<=" => Token::ShlAssign,
            ">>=" => Token::ShrAssign,
            "&^=" => Token::AndNotAssign,
            "&&" => Token::Land,
            "||" => Token::Lor,
            "<-" => Token::Arrow,
            "++" => Token::Inc,
            "--" => Token::Dec,
            "==" => Token::Eql,
            "<" => Token::Lss,
            ">" => Token::Gtr,
            "=" => Token::Assign,
            "!" => Token::Not,
            "!=" => Token::Neq,
            "<=" => Token::Leq,
            ">=" => Token::Geq,
            ":=" => Token::Define,
            "..." => Token::Ellipsis,
            "(" => Token::Lparen,
            "[" => Token::Lbrack,
            "{" => Token::Lbrace,
            "," => Token::Comma,
            "." => Token::Period,
            ")" => Token::Rparen,
            "]" => Token::Rbrack,
            "}" => Token::Rbrace,
            ";" => Token::Semicolon,
            ":" => Token::Colon,
            "~" => Token::Tilde,
            "break" => Token::Break,
            "case" => Token::Case,
            "chan" => Token::Chan,
            "const" => Token::Const,
            "continue" => Token::Continue,
            "default" => Token::Default,
            "defer" => Token::Defer,
            "else" => Token::Else,
            "fallthrough" => Token::Fallthrough,
            "for" => Token::For,
            "func" => Token::Func,
            "go" => Token::Go,
            "goto" => Token::Goto,
            "if" => Token::If,
            "import" => Token::Import,
            "interface" => Token::Interface,
            "map" => Token::Map,
            "package" => Token::Package,
            "range" => Token::Range,
            "return" => Token::Return,
            "select" => Token::Select,
            "struct" => Token::Struct,
            "switch" => Token::Switch,
            "type" => Token::Type,
            "var" => Token::Var,
            refused => return Err(Unspelled(refused)),
        })
    }
}

/// The last child of `node`: the delimiter that closes a list.
pub(super) fn closing<'tree>(node: &Library<'tree>) -> Option<Library<'tree>> {
    node.child(u32::try_from(node.child_count().saturating_sub(1)).unwrap_or_default())
}

/// Whether the list `node` ends with a line break after its last element and no comma there: Go makes the line break a `;`, and
/// refuses a list that has one where it wants a comma.
fn lacks_comma(node: &Library<'_>) -> bool {
    let Some(closing) = closing(node) else {
        return false;
    };
    let mut last = closing.prev_sibling();
    while last.is_some_and(|each| each.kind() == "comment") {
        last = last.and_then(|each| each.prev_sibling());
    }
    last.is_some_and(|last| {
        last.is_named() && closing.start_position().row > last.end_position().row
    })
}

/// What the library reads at `node` that Go does not allow, and the library does not call an error.
pub(super) fn misreading(node: &Library<'_>) -> Misreading {
    let outside = node.is_named()
        && node
            .parent()
            .is_some_and(|parent| parent.kind() == "source_file")
        && !matches!(
            node.kind(),
            "package_clause"
                | "import_declaration"
                | "const_declaration"
                | "var_declaration"
                | "type_declaration"
                | "function_declaration"
                | "method_declaration"
                | "empty_statement"
                | "comment"
                | "ERROR"
        );
    if outside {
        Misreading::Statement
    } else if matches!(
        node.kind(),
        "literal_value" | "argument_list" | "parameter_list"
    ) && lacks_comma(node)
    {
        Misreading::Comma
    } else {
        Misreading::Fits
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
