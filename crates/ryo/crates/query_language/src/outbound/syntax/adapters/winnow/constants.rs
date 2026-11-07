//! The names of the winnow adapter: the words and marks RyoQL is written with. Text enters the
//! engine here; the parsers match on these variants, never on text of their own.

/// A word of the grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Keyword {
    /// `param`.
    Param,
    /// `import`.
    Import,
    /// `input`.
    Input,
    /// `output`.
    Output,
    /// `return`.
    Return,
    /// `relation`.
    Relation,
    /// `let`.
    Let,
    /// `emit`.
    Emit,
    /// `observe`.
    Observe,
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
    /// `numeric_gte`.
    NumericGte,
    /// `integer_successor`.
    IntegerSuccessor,
    /// `starts_with`.
    StartsWith,
    /// `matches`.
    Matches,
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
    /// `lower`.
    Lower,
    /// `natural_lt`.
    NaturalLt,
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
        Keyword::Import => "import",
        Keyword::Input => "input",
        Keyword::Output => ryo_values::domain::descriptions::constants::OUTPUT,
        Keyword::Return => "return",
        Keyword::Relation => ryo_values::domain::descriptions::constants::RELATION,
        Keyword::Let => "let",
        Keyword::Emit => "emit",
        Keyword::Observe => "observe",
        Keyword::Subject => "subject",
        Keyword::Key => "key",
        Keyword::Select => ryo_values::domain::descriptions::constants::SELECT,
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
        Keyword::NaturalLt => "natural_lt",
        Keyword::FloatTextGte => "float_text_gte",
        Keyword::NumericGte => "numeric_gte",
        Keyword::IntegerSuccessor => "integer_successor",
        Keyword::StartsWith => "starts_with",
        Keyword::Matches => "matches",
        Keyword::EndsWith => "ends_with",
        Keyword::Concat => "concat",
        Keyword::Pascal => "pascal",
        Keyword::Camel => "camel",
        Keyword::Snake => "snake",
        Keyword::Kebab => "kebab",
        Keyword::Lower => "lower",
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
    "expected an `import`, `param`, `let`, `observe` or `emit` line"
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
        form: "line { line }, and exactly one emit_line",
        example: "let found = select * from \"function\"; emit found subject \"file\" key(\"span\");",
    },
    GrammarLine {
        name: "line",
        form: "import_line | parameter_line | step_line | observe_line | emit_line",
        example: "let found = select * from \"function\";",
    },
    GrammarLine {
        name: "import_line",
        form: "import word from string \";\"",
        example: "import suppress from \"shared/suppress\";",
    },
    GrammarLine {
        name: "fragment_program",
        form: "import_line | fragment_input_line | parameter_line | fragment_output_line | step_line | fragment_return_line; exactly one output and return",
        example: "input rows: relation<id: node>; output relation<id: node>; let result = select id from rows; return result;",
    },
    GrammarLine {
        name: "fragment_line",
        form: "import_line | fragment_input_line | parameter_line | fragment_output_line | step_line | fragment_return_line",
        example: "input rows: relation<id: node>;",
    },
    GrammarLine {
        name: "fragment_input_line",
        form: "input word \":\" relation_signature \";\"",
        example: "input rows: relation<id: node>;",
    },
    GrammarLine {
        name: "fragment_output_line",
        form: "output relation_signature \";\"",
        example: "output relation<id: node>;",
    },
    GrammarLine {
        name: "relation_signature",
        form: "relation \"<\" word \":\" word { \",\" word \":\" word } \">\"",
        example: "relation<id: node, file: file>",
    },
    GrammarLine {
        name: "fragment_return_line",
        form: "return word \";\"",
        example: "return result;",
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
        name: "observe_line",
        form: "observe word as name_or_string \";\"",
        example: "observe matched as \"usage\";",
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
        name: ryo_values::domain::descriptions::constants::SELECT,
        form: "select ( \"*\" | select_column { \",\" select_column } ) from source [ join ] [ filters ]",
        example: "select l.name as \"name\" from \"function\" as l join \"function\" as r on l.file = r.file where r.name != \"main\"",
    },
    GrammarLine {
        name: "select_column",
        form: "column_reference [ as name_or_string ]",
        example: "l.name as \"name\"",
    },
    GrammarLine {
        name: ryo_relations::domain::relations::constants::SOURCE,
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
        form: "glob_filter | float_text_filter | numeric_filter | integer_successor_filter | affix_filter | comparison_filter",
        example: "name = \"main\"",
    },
    GrammarLine {
        name: "float_text_filter",
        form: "float_text_gte \"(\" operand \",\" operand \")\"",
        example: "float_text_gte(text, $limit)",
    },
    GrammarLine {
        name: "numeric_filter",
        form: "numeric_gte \"(\" operand \",\" operand \")\"",
        example: "numeric_gte(value, 86400)",
    },
    GrammarLine {
        name: "integer_successor_filter",
        form: "integer_successor \"(\" operand \",\" operand \")\"",
        example: "integer_successor(lower, next_lower)",
    },
    GrammarLine {
        name: "glob_filter",
        form: "[ not ] glob_any \"(\" operand \",\" operand \")\"",
        example: "not glob_any(path, $allowed)",
    },
    GrammarLine {
        name: "affix_filter",
        form: "( natural_lt | starts_with | ends_with | matches ) \"(\" operand \",\" operand \")\", a text prefix, suffix or regular-expression search",
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
        form: "( concat | pascal | camel | snake | kebab | lower ) \"(\" operand { \",\" operand } \")\", each operand text: concat joins two or more; pascal, camel, snake and kebab take one, todo_lists as TodoLists, todoLists, todo_lists and todo-lists",
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
        name: ryo_relations::domain::relations::constants::NUMBER,
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
