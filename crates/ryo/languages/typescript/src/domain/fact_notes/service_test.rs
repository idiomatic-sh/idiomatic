//! The `fact_notes` service's cases: each module a file imports by a string is noted with the place of what imports it, as the notes the
//! parse leaves of the file are read back.

use super::types::Note;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::projects::notes_of;
use crate::outbound::notes::factory::postcard;
use crate::outbound::notes::interface::NotesPort as _;

/// What a case starts from: a file, its path and its text: its notes, read back, each import shown as `specifier start..end`.
#[derive(Clone, Copy, Debug)]
struct Given(&'static str, &'static str);

type Input = Given;
type Value = &'static [&'static str];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_module_imported_by_a_string_is_noted_with_its_place"),
        word: Word::Succeeds,
        input: || {
            Given(
                "item.ts",
                "import a from './a';\nexport * from \"../b\";\nexport { c } from './c.js';\nimport d = require('d');\nconst e = import('./e');\nexport const f = 1;\nimport type { G } from './g';\n",
            )
        },
        expect: Expect::Value(&[
            "./a 0..20",
            "../b 21..42",
            "./c.js 43..70",
            "d 71..95",
            "./e 106..119",
            "./g 141..170",
        ]),
    },
    Case {
        name: CaseName("a_script_block_s_imports_are_noted_at_their_place_in_the_file"),
        word: Word::Succeeds,
        input: || {
            Given(
                "item.vue",
                "<template><p>hi</p></template>\n<script lang=\"ts\">\nimport a from './a';\n</script>\n",
            )
        },
        expect: Expect::Value(&["./a 50..70"]),
    },
    Case {
        name: CaseName("a_file_with_no_import_is_noted_with_none"),
        word: Word::Succeeds,
        input: || Given("item.ts", "const x = 1;\n"),
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

fn subject(Given(path, text): Input) -> Result<Value, Failure> {
    let note = postcard().note_in(&notes_of(path, text)).map_err(drop)?;
    Ok(leaked(shown(&note)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
