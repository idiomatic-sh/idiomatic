//! The types of the `fields` concept: plain data, public fields and derives, with no constructor and no inherent impl. A field is the role
//! of an element under its parent, in the words of `ESTree`: the name of the property of the parent node that holds the child, in kebab
//! case. One closed list, each name once, `None` first: the number `0` is the role of an edge that has none.

use crate::domain::nodes::types::Node;

/// The role of an element under its parent.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
pub(crate) enum Field {
    /// The edge has no role: a token, or a child the `ESTree` node holds in no property.
    #[default]
    None,
    /// `alternate`: the branch taken when the test fails, `c` of `a ? b : c` and the `else` of an `if`.
    Alternate,
    /// `argument`: what an operator, a statement or an element applies to, `x` of `return x`, `-x`, `...x`.
    Argument,
    /// `arguments`: the arguments of a call, `a` and `b` of `f(a, b)`.
    Arguments,
    /// `attributes`: the import attributes of a module declaration, or the attributes of a JSX element.
    Attributes,
    /// `block`: the block a `try` runs.
    Block,
    /// `body`: the body of a declaration, a function, a loop or a block.
    Body,
    /// `callee`: what a call or a `new` calls, `f` of `f(x)`.
    Callee,
    /// `cases`: the `case` and `default` clauses of a `switch`.
    Cases,
    /// `children`: the children of a JSX element or fragment.
    Children,
    /// `closingElement`: the closing tag of a JSX element.
    ClosingElement,
    /// `closingFragment`: the closing `</>` of a JSX fragment.
    ClosingFragment,
    /// `consequent`: the branch taken when the test holds, and the statements of a `case`.
    Consequent,
    /// `declaration`: the declaration an export declares.
    Declaration,
    /// `declarations`: the declarators of a variable declaration.
    Declarations,
    /// `discriminant`: the value a `switch` compares.
    Discriminant,
    /// `elements`: the elements of an array or of an array pattern.
    Elements,
    /// `exported`: the name an export gives.
    Exported,
    /// `expression`: the expression a statement or a wrapper holds.
    Expression,
    /// `expressions`: the expressions of a sequence or the substitutions of a template literal.
    Expressions,
    /// `finalizer`: the `finally` block of a `try`.
    Finalizer,
    /// `handler`: the `catch` clause of a `try`.
    Handler,
    /// `id`: the name a declaration binds.
    Id,
    /// `imported`: the name an import specifier takes from the module.
    Imported,
    /// `init`: the initializer of a declarator, or the first clause of a `for`.
    Init,
    /// `key`: the key of a property, a class member or an import attribute.
    Key,
    /// `label`: the label of a labeled statement, a `break` or a `continue`.
    Label,
    /// `left`: the left operand of a binary, logical or assignment expression, or the binding of a `for-in` or `for-of`.
    Left,
    /// `local`: the name a specifier binds in the module that writes it.
    Local,
    /// `meta`: the `import` or `new` of a meta property.
    Meta,
    /// `name`: the name of a JSX element or attribute.
    Name,
    /// `namespace`: the namespace of a JSX namespaced name.
    Namespace,
    /// `object`: the object a member expression reads, or a `with` statement opens.
    Object,
    /// `openingElement`: the opening tag of a JSX element.
    OpeningElement,
    /// `openingFragment`: the opening `<>` of a JSX fragment.
    OpeningFragment,
    /// `operator`: the token that spells the operator of a unary, update, binary, logical or assignment expression.
    Operator,
    /// `options`: the options of an import expression.
    Options,
    /// `param`: the binding of a `catch` clause.
    Param,
    /// `params`: the parameters of a function.
    Params,
    /// `properties`: the properties of an object or of an object pattern.
    Properties,
    /// `property`: the property a member expression reads, or of a meta property.
    Property,
    /// `quasi`: the template of a tagged template.
    Quasi,
    /// `quasis`: the text parts of a template literal.
    Quasis,
    /// `right`: the right operand of a binary, logical or assignment expression, or what a `for-in` or `for-of` iterates.
    Right,
    /// `source`: the module specifier of an import or export declaration, or of an import expression.
    Source,
    /// `specifiers`: the specifiers of an import or export declaration.
    Specifiers,
    /// `superClass`: the class a class extends.
    SuperClass,
    /// `tag`: the function a tagged template calls.
    Tag,
    /// `test`: the condition of a conditional, an `if`, a loop or a `case`.
    Test,
    /// `update`: the last clause of a `for`.
    Update,
    /// `value`: the value of a property, a member, an import attribute or a JSX attribute.
    Value,
}

/// The spelling of a token that marks where a property of its parent begins: the `(` before the parameters of a function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Marker(pub &'static str);

/// Where a node stands among the nodes its parent holds by their place, from zero.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Ordinal(pub usize);

/// Whether a node is the last its parent holds by its place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Lastness {
    /// No node comes after it.
    Last,
    /// A node comes after it.
    NotLast,
}

/// Where a node stands among the nodes its parent holds by their place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Standing {
    /// Its place, from zero.
    pub ordinal: Ordinal,
    /// Whether it is the last.
    pub lastness: Lastness,
}

/// A kind of node and the role a child of that kind plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KindRole {
    /// The kind of the child.
    pub node: Node,
    /// The role it plays.
    pub field: Field,
}

/// A token and the role of the children that come after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MarkerRole {
    /// The token's spelling.
    pub marker: Marker,
    /// The role of the children after it.
    pub field: Field,
}

/// Whether a token of a spelling comes before a child among its siblings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Precedence {
    /// One does.
    Preceded,
    /// None does.
    Unpreceded,
}

/// How many operands an expression with an operator has, which says where its operator stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operands {
    /// Two: the operator follows the left one.
    Binary,
    /// One: the operator is its only token besides parentheses.
    Unary,
}
