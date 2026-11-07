//! The `fact_notes` service's cases: each module a file imports by a string is noted with the place of what imports it, as the parse
//! leaves the notes and the port `notes` reads them back.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::fact_notes::types::Note;
use crate::fixtures::projects::notes_of;
use crate::outbound::notes::factory::postcard;
use crate::outbound::notes::interface::NotesPort as _;

/// A file, its path and its text: its notes, read back, each import shown as `specifier start..end`.
type Input = (&'static str, &'static str);
type Value = &'static [&'static str];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_module_imported_by_a_string_is_noted_with_its_place"),
        word: Word::Succeeds,
        input: || {
            (
                "item.js",
                "import a from './a';\nexport * from \"../b\";\nexport { c } from './c.js';\nconst d = require('d');\nconst e = import('./e');\nexport const f = 1;\nrequire(name);\nother('./x');\n",
            )
        },
        expect: Expect::Value(&[
            "./a 0..20",
            "../b 21..42",
            "./c.js 43..70",
            "d 81..93",
            "./e 105..118",
        ]),
    },
    Case {
        name: CaseName("a_script_block_s_imports_are_noted_at_their_place_in_the_file"),
        word: Word::Succeeds,
        input: || {
            (
                "item.vue",
                "<template><p>hi</p></template>\n<script>\nimport a from './a';\n</script>\n",
            )
        },
        expect: Expect::Value(&["./a 40..60"]),
    },
    Case {
        name: CaseName("a_file_with_no_import_is_noted_with_none"),
        word: Word::Succeeds,
        input: || ("item.js", "const x = 1;\n"),
        expect: Expect::Value(&[]),
    },
];

/// Each import of `note`, as `specifier start..end`.
fn shown(note: &Note) -> Vec<String> {
    note.imports
        .iter()
        .map(|import| {
            format!(
                "{} {}..{}",
                import.specifier.0, import.start.0, import.end.0
            )
        })
        .collect()
}

/// `lines`, leaked: a case compares them with text that lives for the whole run.
fn leaked(lines: Vec<String>) -> Value {
    let lines: Vec<&'static str> = lines
        .into_iter()
        .map(|line| &*Box::leak(line.into_boxed_str()))
        .collect();
    Box::leak(lines.into_boxed_slice())
}

fn subject((path, text): Input) -> Result<Value, Failure> {
    let note = postcard()
        .note_in(&notes_of(path, text))
        .map_err(|_not_a_note| ())?;
    Ok(leaked(shown(&note)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
