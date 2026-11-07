//! The names and the text the `cssparser` adapter reads with.

/// What the adapter says of bytes that are not UTF-8.
pub(super) const NOT_UTF8: &str = "invalid UTF-8: the language reads a stylesheet as UTF-8 text, and the parser reads it no further";

/// The byte order mark a stylesheet may start with, which the byte stream drops before the tokenizer reads.
pub(super) const BYTE_ORDER_MARK: char = '\u{feff}';

/// The at-rule the stylesheet's encoding is, valid only as its first rule.
pub(super) const CHARSET: &str = "charset";

/// The prefix a custom property's name starts with; its value may hold any block.
pub(super) const CUSTOM_PROPERTY: &str = "--";

/// The at-rules whose block holds rules and declarations, as CSS defines them (conditional rules, `@layer`, `@scope`, `@font-face`,
/// `@page` and its margin boxes, `@keyframes`, `@counter-style`, `@property`, the font feature blocks and the rest), compared
/// without regard to case and to a vendor prefix. The block of any other at-rule is read as values.
pub(crate) const RULE_BODIES: &[&str] = &[
    "media",
    "supports",
    "container",
    "layer",
    "scope",
    "starting-style",
    "document",
    "font-face",
    "page",
    "keyframes",
    "counter-style",
    "property",
    "font-feature-values",
    "font-palette-values",
    "view-transition",
    "position-try",
    "viewport",
    "swash",
    "annotation",
    "ornaments",
    "stylistic",
    "styleset",
    "character-variant",
    "top-left-corner",
    "top-left",
    "top-center",
    "top-right",
    "top-right-corner",
    "bottom-left-corner",
    "bottom-left",
    "bottom-center",
    "bottom-right",
    "bottom-right-corner",
    "left-top",
    "left-middle",
    "left-bottom",
    "right-top",
    "right-middle",
    "right-bottom",
];

/// What the language says of a url the tokenizer could not read.
pub(super) const BAD_URL: &str =
    "a url the tokenizer could not read: it holds a quote, a parenthesis or white space inside";

/// What the language says of a string a line end breaks.
pub(super) const BAD_STRING: &str = "a string a line end breaks before its closing quote";

/// What the language says of a `)` no `(` opened.
pub(super) const UNMATCHED_PARENTHESIS: &str = "a `)` that closes no block";

/// What the language says of a `]` no `[` opened.
pub(super) const UNMATCHED_BRACKET: &str = "a `]` that closes no block";

/// What the language says of a `}` no `{` opened.
pub(super) const UNMATCHED_BRACE: &str = "a `}` that closes no block";

/// What the language says of an error its own rule parsers raise, which none does.
pub(super) const REFUSED: &str = "a rule the language refused";

/// The at-rule that imports a stylesheet, as the library decodes its name.
pub(crate) const IMPORT: &str = "import";

/// The function that writes a url in quotes, `url("a.png")`, as the library decodes its name.
pub(super) const URL: &str = "url";
