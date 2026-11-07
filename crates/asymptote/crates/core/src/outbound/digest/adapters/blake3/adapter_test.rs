//! The BLAKE3 adapter of the digest port: the contract, the fingerprint of a known line, and how a
//! violation's identity answers each way real code changes, as the item fingerprint pins it.

use super::adapter::DigestBlake3Adapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::violations::types::SourceText;
use crate::outbound::digest::fixtures::contract::check;
use crate::outbound::digest::interface::DigestPort as _;

/// An impl whose method has a violation on its signature (line 2) and one in its body (line 3), beside an unrelated
/// function.
const IMPL: &str = concat!(
    "impl Store for Disk {\n",
    "    pub fn read(&self) -> Result<String, Error> {\n",
    "        let text = self.file.unwrap();\n",
    "        Ok(text)\n",
    "    }\n",
    "}\n",
    "\n",
    "fn helper() -> u32 {\n",
    "    1\n",
    "}\n",
);

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The contract.
    Contract,
    /// `fn main() {}` at the top level, whose fingerprint is pinned.
    KnownLine,
    /// Lines inserted above.
    Shifted,
    /// An unrelated function renamed.
    Unrelated,
    /// The same text in two functions.
    TwoFunctions,
    /// The enclosing function renamed.
    Renamed,
    /// A multi-line signature's two bodies.
    MultiLine,
    /// A Rust line inside a function, and a Python one, keyed by their item.
    KnownNested,
    /// The same text in two JavaScript functions.
    JavaScriptFunctions,
    /// The same text in two Go functions.
    GoFunctions,
    /// The same text in two Go methods of two receivers.
    GoMethods,
    /// The same text in two methods of a TypeScript class.
    ClassMethods,
    /// The same text in two functions bound to a `const`, one of them `async`.
    BoundFunctions,
    /// A call taking a callback, `it("...", function () {`, encloses nothing: its two bodies are one item's.
    CallbackCall,
    /// The same text in two functions whose bodies are not indented.
    Unindented,
    /// The same text after the closing brace of two unindented functions encloses nothing.
    AfterUnindented,
}

type Input = Question;
/// Whether the answer is the expected one.
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_adapter_keeps_the_contract"),
        word: Word::Succeeds,
        input: || Question::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_top_level_line_is_fingerprinted_as_the_first_asymptote_did"),
        word: Word::Succeeds,
        input: || Question::KnownLine,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("lines_inserted_above_keep_every_fingerprint"),
        word: Word::Succeeds,
        input: || Question::Shifted,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("renaming_an_unrelated_function_keeps_every_fingerprint"),
        word: Word::Succeeds,
        input: || Question::Unrelated,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_same_text_in_another_function_is_another_violation"),
        word: Word::Succeeds,
        input: || Question::TwoFunctions,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("renaming_the_enclosing_function_changes_a_violation_in_its_body"),
        word: Word::Succeeds,
        input: || Question::Renamed,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_multi_line_signature_encloses_its_body"),
        word: Word::Succeeds,
        input: || Question::MultiLine,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("rust_and_python_lines_keep_the_first_asymptotes_keys"),
        word: Word::Succeeds,
        input: || Question::KnownNested,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_same_text_in_another_javascript_function_is_another_violation"),
        word: Word::Succeeds,
        input: || Question::JavaScriptFunctions,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_same_text_in_another_go_function_is_another_violation"),
        word: Word::Succeeds,
        input: || Question::GoFunctions,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_same_text_in_a_method_of_another_go_receiver_is_another_violation"),
        word: Word::Succeeds,
        input: || Question::GoMethods,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_same_text_in_another_class_method_is_another_violation"),
        word: Word::Succeeds,
        input: || Question::ClassMethods,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("the_same_text_in_another_bound_function_is_another_violation"),
        word: Word::Succeeds,
        input: || Question::BoundFunctions,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_call_taking_a_callback_declares_no_item"),
        word: Word::Succeeds,
        input: || Question::CallbackCall,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_body_that_is_not_indented_still_belongs_to_its_function"),
        word: Word::Succeeds,
        input: || Question::Unindented,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName(
            "a_line_after_the_closing_brace_of_an_unindented_body_belongs_to_no_function",
        ),
        word: Word::Succeeds,
        input: || Question::AfterUnindented,
        expect: Expect::Value(true),
    },
];

/// The fingerprint of `line` inside the item `keyword name`.
fn keyed(line: &[u8], keyword: &[u8], name: &[u8]) -> u64 {
    let mut prefix = [0_u8; 8];
    blake3::Hasher::new()
        .update(line)
        .update(b"\0")
        .update(keyword)
        .update(name)
        .finalize_xof()
        .fill(&mut prefix);
    u64::from_le_bytes(prefix)
}

fn print(source: &str, line: usize) -> u64 {
    DigestBlake3Adapter.lines(&SourceText(source.into())).0[line - 1].0
}

fn subject(question: Input) -> Value {
    match question {
        Question::Contract => {
            check(&DigestBlake3Adapter);
            true
        }
        Question::KnownLine => {
            let mut prefix = [0_u8; 8];
            blake3::Hasher::new()
                .update(b"fnmain(){}\0")
                .finalize_xof()
                .fill(&mut prefix);
            print("fn main() {}\n", 1) == u64::from_le_bytes(prefix)
        }
        Question::Shifted => {
            let shifted = format!("//! A module.\n\nuse std::fmt;\n\n{IMPL}");
            print(IMPL, 2) == print(&shifted, 6) && print(IMPL, 3) == print(&shifted, 7)
        }
        Question::Unrelated => {
            let renamed = IMPL.replace("fn helper", "fn assist");
            print(IMPL, 2) == print(&renamed, 2) && print(IMPL, 3) == print(&renamed, 3)
        }
        Question::TwoFunctions => {
            let source =
                "fn first() {\n    value.unwrap();\n}\n\nfn second() {\n    value.unwrap();\n}\n";
            print(source, 2) != print(source, 6)
        }
        Question::Renamed => print(IMPL, 3) != print(&IMPL.replace("fn read", "fn load"), 3),
        Question::MultiLine => {
            let source = "pub fn run(\n    a: u32,\n) -> u32 {\n    a.unwrap()\n}\n\npub fn walk(\n    a: u32,\n) -> u32 {\n    a.unwrap()\n}\n";
            print(source, 4) != print(source, 10)
        }
        Question::KnownNested => {
            print("fn first() {\n    value.unwrap();\n}\n", 2)
                == keyed(b"value.unwrap();", b"fn", b"first")
                && print("def first():\n    x = None\n", 2) == keyed(b"x=None", b"def", b"first")
                && print("impl<T> Store for Disk<T> where T: Copy {\n    x\n}\n", 2)
                    == keyed(b"x", b"impl", b"<T>StoreforDisk<T>")
        }
        Question::JavaScriptFunctions => {
            let source = "function first() {\n    bad();\n}\nfunction second() {\n    bad();\n}\n";
            let exported = "export async function first() {\n    bad();\n}\nexport function* second() {\n    bad();\n}\n";
            print(source, 2) != print(source, 5) && print(exported, 2) != print(exported, 5)
        }
        Question::GoFunctions => {
            let source = "func first() {\n\tbad()\n}\n\nfunc second() {\n\tbad()\n}\n";
            print(source, 2) != print(source, 6)
        }
        Question::GoMethods => {
            let source = "func (s *Server) Start() {\n\tbad()\n}\n\nfunc (c *Client) Start() {\n\tbad()\n}\n";
            print(source, 2) != print(source, 6)
        }
        Question::ClassMethods => {
            let source = "export class Store {\n  constructor(a: number) {\n    bad();\n  }\n  public async load(): Promise<void> {\n    bad();\n  }\n}\n";
            print(source, 3) != print(source, 6)
        }
        Question::BoundFunctions => {
            let source = "const first = () => {\n  bad();\n};\nexport const second = async (a: number): Promise<void> => {\n  bad();\n};\n";
            print(source, 2) != print(source, 5)
        }
        Question::Unindented => {
            let source = "function first() {\nbad();\n}\nfunction second() {\nbad();\n}\n";
            print(source, 2) != print(source, 5)
                && print(source, 2) == keyed(b"bad();", b"function", b"first")
        }
        Question::AfterUnindented => {
            let source =
                "function first() {\nbad();\n}\nbad();\nfunction second() {\nbad();\n}\nbad();\n";
            print(source, 4) == print(source, 8) && print(source, 2) != print(source, 6)
        }
        Question::CallbackCall => {
            let source = "function suite() {\n  it(\"one\", function () {\n    bad();\n  });\n  it(\"two\", function () {\n    bad();\n  });\n}\n";
            print(source, 3) == print(source, 6)
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
