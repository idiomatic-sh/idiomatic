//! The names of the winnow adapter: the words and marks RyoQL is written with. Text enters the
//! engine here; the parsers match on these variants, never on text of their own.

/// A word of the grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keyword {
    /// `param`.
    Param,
    /// `let`.
    Let,
    /// `emit`.
    Emit,
    /// `subject`.
    Subject,
    /// `key`.
    Key,
    /// `select`.
    Select,
    /// `from`.
    From,
    /// `as`.
    As,
    /// `join`.
    Join,
    /// `on`.
    On,
    /// `where`.
    Where,
    /// `and`.
    And,
    /// `or`.
    Or,
    /// `not`.
    Not,
    /// `in`.
    In,
    /// `glob_any`.
    GlobAny,
    /// `float_text_gte`.
    FloatTextGte,
    /// `starts_with`.
    StartsWith,
    /// `ends_with`.
    EndsWith,
    /// `concat`.
    Concat,
    /// `pascal`.
    Pascal,
    /// `camel`.
    Camel,
    /// `snake`.
    Snake,
    /// `kebab`.
    Kebab,
}

/// A mark of the grammar made of more than one character.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mark {
    /// `!=`.
    Different,
    /// `<=`.
    AtMost,
    /// `>=`.
    AtLeast,
}

/// The text of a keyword.
#[must_use]
pub(crate) fn keyword_text(keyword: Keyword) -> &'static str {
    match keyword {
        Keyword::Param => "param",
        Keyword::Let => "let",
        Keyword::Emit => "emit",
        Keyword::Subject => "subject",
        Keyword::Key => "key",
        Keyword::Select => crate::domain::syntax::constants::SELECT,
        Keyword::From => "from",
        Keyword::As => "as",
        Keyword::Join => "join",
        Keyword::On => "on",
        Keyword::Where => "where",
        Keyword::And => "and",
        Keyword::Or => "or",
        Keyword::Not => "not",
        Keyword::In => "in",
        Keyword::GlobAny => "glob_any",
        Keyword::FloatTextGte => "float_text_gte",
        Keyword::StartsWith => "starts_with",
        Keyword::EndsWith => "ends_with",
        Keyword::Concat => "concat",
        Keyword::Pascal => "pascal",
        Keyword::Camel => "camel",
        Keyword::Snake => "snake",
        Keyword::Kebab => "kebab",
    }
}

/// The text of a mark.
#[must_use]
pub(crate) fn mark_text(mark: Mark) -> &'static str {
    match mark {
        Mark::Different => "!=",
        Mark::AtMost => "<=",
        Mark::AtLeast => ">=",
    }
}

/// What the grammar expects when a text stops making sense and winnow said nothing more.
#[must_use]
pub(crate) fn expectation() -> &'static str {
    "expected a `param`, `let` or `emit` line"
}

/// One production of the grammar as this adapter reads it: the parser fn that reads it, how it is written, and text it reads whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct GrammarLine {
    /// The parser fn that reads it.
    pub name: &'static str,
    /// How it is written: quoted text is written as is, a bare lowercase word is a keyword or another production.
    pub form: &'static str,
    /// Text the parser fn reads to its end.
    pub example: &'static str,
}

/// The grammar of RyoQL, one line for each parser fn of the adapter, from the whole program down to a word.
pub(crate) const GRAMMAR: &[GrammarLine] = &[
    GrammarLine {
        name: "program",
        form: "line { line }, with exactly one emit_line",
        example: "let found = select * from \"function\"; emit found subject \"file\" key(\"span\");",
    },
    GrammarLine {
        name: "line",
        form: "parameter_line | step_line | emit_line",
        example: "let found = select * from \"function\";",
    },
    GrammarLine {
        name: "parameter_line",
        form: "param word \":\" type \";\" with type boolean, integer, string, path, glob, list<...>, set<...> or list<table>",
        example: "param max-lines: integer;",
    },
    GrammarLine {
        name: "step_line",
        form: "let word \"=\" expression \";\"",
        example: "let found = select * from \"function\";",
    },
    GrammarLine {
        name: "emit_line",
        form: "emit word subject name_or_string key \"(\" name_or_string { \",\" name_or_string } \")\" \";\"",
        example: "emit found subject \"file\" key(\"span\");",
    },
    GrammarLine {
        name: "expression",
        form: "select | call",
        example: "select * from \"function\"",
    },
    GrammarLine {
        name: crate::domain::syntax::constants::SELECT,
        form: "select ( \"*\" | select_column { \",\" select_column } ) from source [ join ] [ filters ]",
        example: "select l.name as \"name\" from \"function\" as l join \"function\" as r on l.file = r.file where r.name != \"main\"",
    },
    GrammarLine {
        name: "select_column",
        form: "column_reference [ as name_or_string ]",
        example: "l.name as \"name\"",
    },
    GrammarLine {
        name: "source",
        form: "( string | word ) [ as word ], a string naming a relation and a word a step",
        example: "\"function\" as l",
    },
    GrammarLine {
        name: "join",
        form: "join source on join_key { and join_key }",
        example: " join \"function\" as r on l.file = r.file",
    },
    GrammarLine {
        name: "join_key",
        form: "column_reference \"=\" column_reference",
        example: "l.file = r.file",
    },
    GrammarLine {
        name: "filters",
        form: "where condition; after a join, alias.column names a column of that side, kept or not",
        example: " where name = \"main\" and line > 1",
    },
    GrammarLine {
        name: "condition",
        form: "conjunction { or conjunction }, and binding tighter than or",
        example: "kind = \"a\" or kind = \"b\" and line > 1",
    },
    GrammarLine {
        name: "conjunction",
        form: "factor { and factor }",
        example: "name = \"main\" and (line > 1 or line < 0)",
    },
    GrammarLine {
        name: "factor",
        form: "\"(\" condition \")\" | filter",
        example: "(kind = \"a\" or kind = \"b\")",
    },
    GrammarLine {
        name: "filter",
        form: "glob_filter | float_text_filter | affix_filter | comparison_filter",
        example: "name = \"main\"",
    },
    GrammarLine {
        name: "float_text_filter",
        form: "float_text_gte \"(\" operand \",\" operand \")\"",
        example: "float_text_gte(text, $limit)",
    },
    GrammarLine {
        name: "glob_filter",
        form: "[ not ] glob_any \"(\" operand \",\" operand \")\"",
        example: "not glob_any(path, $allowed)",
    },
    GrammarLine {
        name: "affix_filter",
        form: "( starts_with | ends_with ) \"(\" operand \",\" operand \")\", whether the first text starts or ends with the second",
        example: "ends_with(name, concat(pascal(p.pkg), \"Service\"))",
    },
    GrammarLine {
        name: "comparison_filter",
        form: "operand test operand",
        example: "line >= $first",
    },
    GrammarLine {
        name: "test",
        form: "\"=\" | \"!=\" | \"<\" | \"<=\" | \">\" | \">=\" | in | not in",
        example: "not in",
    },
    GrammarLine {
        name: "operand",
        form: "parameter | string | number | function | column_reference",
        example: "$limit",
    },
    GrammarLine {
        name: "function",
        form: "( concat | pascal | camel | snake | kebab ) \"(\" operand { \",\" operand } \")\", each operand text: concat joins two or more; pascal, camel, snake and kebab take one, todo_lists as TodoLists, todoLists, todo_lists and todo-lists",
        example: "concat(pascal(p.pkg), \"Service\")",
    },
    GrammarLine {
        name: "column_reference",
        form: "[ word \".\" ] name_or_string",
        example: "l.\"span\"",
    },
    GrammarLine {
        name: "call",
        form: "word \"(\" [ argument { \",\" argument } [ \",\" ] ] \")\", word being an operator",
        example: "project(from=found, columns=[\"path\"])",
    },
    GrammarLine {
        name: "argument",
        form: "name_or_string ( \"=\" | \":\" ) value",
        example: "columns=[\"path\"]",
    },
    GrammarLine {
        name: "value",
        form: "string | number | parameter | \"[\" [ value { \",\" value } ] \"]\" | \"{\" [ argument { \",\" argument } ] \"}\" | word",
        example: "{ min: 1, kinds: [\"fn\"] }",
    },
    GrammarLine {
        name: "parameter",
        form: "\"$\" word",
        example: "$max-lines",
    },
    GrammarLine {
        name: "number",
        form: "digit { digit }",
        example: "400",
    },
    GrammarLine {
        name: "name_or_string",
        form: "string | word",
        example: "\"span\"",
    },
    GrammarLine {
        name: "string",
        form: "a double quote, any characters but a double quote, a double quote",
        example: "\"function\"",
    },
    GrammarLine {
        name: "word",
        form: "one or more of a-z A-Z 0-9 \"_\" \"-\"",
        example: "max-lines",
    },
];
