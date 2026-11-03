//! The rows each relation read from trees must give for its fixture, one module for each relation.

#![expect(
    clippy::inline_modules,
    reason = "each relation's expected rows are a file of their own under tests/fixtures, which `--bless` writes; `include!` makes each one a module here, and the module spec declares no file by `#[path]` but a file's own test"
)]

use ryo::languages::Why;
use ryo::languages::contract::{Expected, Unfilled, Unshown};

/// Independently expected canonical lexical names.
mod expression_shape {
    include!("../../tests/fixtures/relations/expression_shape/expected.rs");
}
/// Primitive parsed-literal properties retain their source identity.
mod literal_properties {
    include!("../../tests/fixtures/relations/literal_properties/expected.rs");
}
/// Numeric parser payloads retain real source identity and exact integer values.
mod numeric_literal {
    include!("../../tests/fixtures/relations/numeric_literal/expected.rs");
}
/// Basic per-expression type classifications.
mod expression_type {
    include!("../../tests/fixtures/relations/expression_type/expected.rs");
}
/// Structured terminal outcomes for functions.
mod function_terminal {
    include!("../../tests/fixtures/relations/function_terminal/expected.rs");
}
/// Decoded text fragments are scoped to their owning f-string.
mod format_specification {
    include!("../../tests/fixtures/relations/format_specification/expected.rs");
}
mod bytes_literal {
    include!("../../tests/fixtures/relations/bytes_literal/expected.rs");
}
mod interpolated_string_literal {
    include!("../../tests/fixtures/relations/interpolated_string_literal/expected.rs");
}
/// Parser-target metadata is present even for an empty Python file.
mod language_target {
    include!("../../tests/fixtures/relations/language_target/expected.rs");
}
/// Normalized parser-owned constant expression keys.
mod constant_key {
    include!("../../tests/fixtures/relations/constant_key/expected.rs");
}
/// Percent-format outcomes retain parser status and source identity.
mod format_parse {
    include!("../../tests/fixtures/relations/format_parse/expected.rs");
}
/// Percent-format conversion fields retain argument order.
mod format_field {
    include!("../../tests/fixtures/relations/format_field/expected.rs");
}
/// Raw format token outcomes preserve source identity.
mod raw_format_segment {
    include!("../../tests/fixtures/relations/raw_format_segment/expected.rs");
}
/// Successful raw fields retain both local and global order.
mod raw_format_field {
    include!("../../tests/fixtures/relations/raw_format_field/expected.rs");
}
/// Independently expected identifier spellings.
mod identifier {
    include!("../../tests/fixtures/relations/identifier/expected.rs");
}
/// Independently specified lexical provenance fixtures.
mod qualified_name {
    include!("../../tests/fixtures/relations/qualified_name/expected.rs");
}
/// The rows the `resolves` relation must give for its fixture input.
mod resolves {
    include!("../../tests/fixtures/relations/resolves/expected.rs");
}
/// The rows the `call_argument` relation must give for its fixture input.
mod call_argument {
    include!("../../tests/fixtures/relations/call_argument/expected.rs");
}
/// The rows the `annotates` relation must give for its fixture.
mod annotates {
    include!("../../tests/fixtures/relations/annotates/expected.rs");
}
/// The rows the `assigns` relation must give for its fixture.
mod assigns {
    include!("../../tests/fixtures/relations/assigns/expected.rs");
}
/// The rows the `comment` relation must give for its fixture.
mod comment {
    include!("../../tests/fixtures/relations/comment/expected.rs");
}
/// The rows the `decorates` relation must give for its fixture.
mod decorates {
    include!("../../tests/fixtures/relations/decorates/expected.rs");
}
/// The rows the `defines` relation must give for its fixture.
mod defines {
    include!("../../tests/fixtures/relations/defines/expected.rs");
}
/// The rows the `function` relation must give for its fixture.
mod function {
    include!("../../tests/fixtures/relations/function/expected.rs");
}
/// The rows the `inherits` relation must give for its fixture.
mod inherits {
    include!("../../tests/fixtures/relations/inherits/expected.rs");
}
/// The rows the `parameter` relation must give for its fixture.
mod parameter {
    include!("../../tests/fixtures/relations/parameter/expected.rs");
}
/// The rows the `reference` relation must give for its fixture.
mod reference {
    include!("../../tests/fixtures/relations/reference/expected.rs");
}
/// The rows the `scope` relation must give for its fixture.
mod scope {
    include!("../../tests/fixtures/relations/scope/expected.rs");
}
/// The rows the `static-call` relation must give for its fixture.
mod static_call {
    include!("../../tests/fixtures/relations/static_call/expected.rs");
}
/// The rows the `static-import` relation must give for its fixture.
mod static_import {
    include!("../../tests/fixtures/relations/static_import/expected.rs");
}
/// The rows the `standard-library-import` relation must give for its fixture.
mod standard_library_import {
    include!("../../tests/fixtures/relations/standard_library_import/expected.rs");
}
/// The rows the `string-literal` relation must give for its fixture.
mod string_literal {
    include!("../../tests/fixtures/relations/string_literal/expected.rs");
}
/// The rows the `syntax-child` relation must give for its fixture.
mod syntax_child {
    include!("../../tests/fixtures/relations/syntax_child/expected.rs");
}
/// The rows the `syntax-error` relation must give for its fixture.
mod syntax_error {
    include!("../../tests/fixtures/relations/syntax_error/expected.rs");
}
/// The rows the `syntax-tree` relation must give for its fixture.
mod syntax_tree {
    include!("../../tests/fixtures/relations/syntax_tree/expected.rs");
}

/// Independently expected source coordinates for a minimal input.
mod source_location {
    include!("../../tests/fixtures/relations/source_location/expected.rs");
}

/// The independently authored lexical-region fixture.
mod indent_region {
    include!("../../tests/fixtures/relations/indent_region/expected.rs");
}

/// The expected rows of every relation the language reads from trees, one entry for each.
pub(super) fn expected() -> Vec<Expected> {
    vec![
        binding_event::expected(),
        binding_read::expected(),
        binding_use::expected(),
        binding_shadow::expected(),
        implicit_symbol::expected(),
        brace_format_parse::expected(),
        brace_format_field::expected(),
        annotates::expected(),
        assigns::expected(),
        comment::expected(),
        decorates::expected(),
        defines::expected(),
        function::expected(),
        inherits::expected(),
        parameter::expected(),
        reference::expected(),
        scope::expected(),
        static_call::expected(),
        resolves::expected(),
        qualified_name::expected(),
        identifier::expected(),
        expression_shape::expected(),
        literal_properties::expected(),
        numeric_literal::expected(),
        token_link::expected(),
        literal_expression_parse::expected(),
        expression_type::expected(),
        function_terminal::expected(),
        interpolated_string_literal::expected(),
        bytes_literal::expected(),
        format_specification::expected(),
        language_target::expected(),
        compiler_feature::expected(),
        constant_key::expected(),
        format_parse::expected(),
        format_field::expected(),
        raw_format_segment::expected(),
        raw_format_field::expected(),
        call_argument::expected(),
        static_import::expected(),
        standard_library_import::expected(),
        string_literal::expected(),
        syntax_child::expected(),
        semantic_child::expected(),
        syntax_error::expected(),
        syntax_tree::expected(),
        semantic_syntax::expected(),
        embedded_expression::expected(),
        source_location::expected(),
        indent_region::expected(),
    ]
}

/// The columns of the relations the language reads that its fixtures cannot show filled, each with why it never applies.
pub(super) fn unfilled() -> Vec<Unfilled> {
    let mut found = vec![
        Unfilled {
            relation: "comment",
            column: "doc",
            unshown: Unshown::Word("true"),
            reason: Why(
                "Python documents with a docstring, a string the language reads as `__doc__` (a `string_literal` row): a `#` comment is never documentation",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "catalog",
            unshown: Unshown::Filled,
            reason: Why(
                "no Python manifest takes a requirement from a shared table: PEP 621, Poetry, Pipenv and requirements files write each requirement themselves, and uv's `workspace = true` says where a package comes from (`origin`), its requirement staying in `dependencies`",
            ),
        },
        Unfilled {
            relation: "package_dependency",
            column: "kind",
            unshown: Unshown::Word("peer"),
            reason: Why(
                "Python packaging (PEP 508, PEP 621, PEP 735) has no peer dependency: a package states what it needs, never what its user must install beside it",
            ),
        },
    ];
    found.extend(words(
        "defines",
        "kind",
        &["struct", "enum", "enum-member", "trait", "interface", "type", "namespace", "package", "constant", "macro", "section", "label", "implementation"],
        "the Python Language Reference has no such declaration: `class` declares every type (an enum or a protocol is a class), a name is bound by assignment with no constant form, a package is a directory, and there are no macros, labels or implementations",
    ));
    found.extend(words(
        "defines",
        "kind",
        &["property"],
        "Python declares no field: a name a class body binds is a class variable, a `variable` row (Language Reference 9.3.4), and `@property` is a decorator, a `decorates` row",
    ));
    found.extend(words(
        "parameter",
        "kind",
        &["receiver", "result"],
        "a Python method's `self` is an ordinary first parameter the language does not set apart (Language Reference 8.7), and a function names no results",
    ));
    found
}

/// One declaration for each word of a closed set the language never writes, all for one reason.
fn words(
    relation: &'static str,
    column: &'static str,
    words: &'static [&'static str],
    reason: &'static str,
) -> impl Iterator<Item = Unfilled> {
    words.iter().map(move |word| Unfilled {
        relation,
        column,
        unshown: Unshown::Word(word),
        reason: Why(reason),
    })
}

/// Native syntax is preserved by the semantic projection.
mod semantic_syntax {
    include!("../../tests/fixtures/relations/semantic_syntax/expected.rs");
}

/// Native syntax is preserved by the semantic projection.
mod semantic_child {
    include!("../../tests/fixtures/relations/semantic_child/expected.rs");
}

/// Independent source mapping and owner identity for a quoted annotation.
mod embedded_expression {
    include!("../../tests/fixtures/relations/embedded_expression/expected.rs");
}

/// Syntactic outcomes preserve original carrier identity and span.
mod literal_expression_parse {
    include!("../../tests/fixtures/relations/literal_expression_parse/expected.rs");
}

/// A valid Python module future mode comes from the frontend-owned compiler flag.
mod compiler_feature {
    include!("../../tests/fixtures/relations/compiler_feature/expected.rs");
}

/// Canonical positive fixture for `binding_event`.
mod binding_event {
    include!("../../tests/fixtures/relations/binding_event/expected.rs");
}

/// Canonical positive fixture for `binding_read`.
mod binding_read {
    include!("../../tests/fixtures/relations/binding_read/expected.rs");
}

/// Canonical positive fixture for `binding_use`.
mod binding_use {
    include!("../../tests/fixtures/relations/binding_use/expected.rs");
}

/// Canonical positive fixture for `binding_shadow`.
mod binding_shadow {
    include!("../../tests/fixtures/relations/binding_shadow/expected.rs");
}

/// Canonical positive fixture for `implicit_symbol`.
mod implicit_symbol {
    include!("../../tests/fixtures/relations/implicit_symbol/expected.rs");
}

/// Canonical positive fixture for `brace_format_parse`.
mod brace_format_parse {
    include!("../../tests/fixtures/relations/brace_format_parse/expected.rs");
}

/// Canonical positive fixture for `brace_format_field`.
mod brace_format_field {
    include!("../../tests/fixtures/relations/brace_format_field/expected.rs");
}

/// Optional physical lexical edges over existing token IDs.
mod token_link {
    include!("../../tests/fixtures/relations/token_link/expected.rs");
}
