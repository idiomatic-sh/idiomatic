//! The svelte adapter's cases, and the `embedded` port's contract it runs.

use super::adapter::EmbeddedSvelteAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::embedded::fixtures::contract::check;
use crate::outbound::embedded::interface::EmbeddedPort as _;
use crate::outbound::embedded::types::{BlockLanguage, Markup};

/// What a row asks of the adapter.
#[derive(Clone, Copy, Debug)]
enum Ask {
    /// The blocks found in this text, each as `start:text:dialect`.
    Found(&'static str),
    /// Whether each block's text is the text of the file at its start.
    Aligned(&'static str),
    /// The embedded port's contract, which every adapter holds.
    Contract,
}

type Input = Ask;
type Value = &'static str;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two_script_blocks_are_found_in_file_order"),
        word: Word::Succeeds,
        input: || Ask::Found("<script>a</script>\n<p>x</p>\n<script context=\"module\">b</script>"),
        expect: Expect::Value("8:a:script|53:b:script"),
    },
    Case {
        name: CaseName("every_block_starts_where_its_text_is_in_the_file"),
        word: Word::Succeeds,
        input: || {
            Ask::Aligned("<p>é</p>\n<script>a</script>\n<script context=\"module\">b</script>")
        },
        expect: Expect::Value("aligned"),
    },
    Case {
        name: CaseName("a_typescript_block_and_a_script_block_differ_in_dialect"),
        word: Word::Succeeds,
        input: || Ask::Found("<script lang=\"ts\">a</script><script>b</script>"),
        expect: Expect::Value("18:a:typed|36:b:script"),
    },
    Case {
        name: CaseName("the_lang_attribute_is_read_as_an_attribute_not_as_text"),
        word: Word::Succeeds,
        input: || {
            Ask::Found(
                "<script lang = \"ts\">a</script><script lang=tsx>b</script><script data-lang=\"ts\">c</script>",
            )
        },
        expect: Expect::Value("20:a:typed|47:b:tsx|80:c:script"),
    },
    Case {
        name: CaseName("a_file_with_no_script_has_no_block"),
        word: Word::Succeeds,
        input: || Ask::Found("<template>x</template>"),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_tag_that_only_starts_like_script_is_not_a_block"),
        word: Word::Succeeds,
        input: || Ask::Found("<scripts>x</scripts>"),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_script_in_a_comment_is_not_a_block"),
        word: Word::Succeeds,
        input: || Ask::Found("<!-- <script>a</script> -->"),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_self_closing_script_has_no_text"),
        word: Word::Succeeds,
        input: || Ask::Found("<script src=\"a.js\" />"),
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("the_adapter_holds_the_embedded_contract"),
        word: Word::Succeeds,
        input: || Ask::Contract,
        expect: Expect::Value("held"),
    },
];

fn subject(ask: Input) -> Value {
    match ask {
        Ask::Found(text) => Box::leak(
            EmbeddedSvelteAdapter
                .blocks(Markup(text))
                .0
                .iter()
                .map(|block| {
                    format!(
                        "{}:{}:{}",
                        block.start.0,
                        block.text.0,
                        match block.dialect {
                            BlockLanguage::Script => "script",
                            BlockLanguage::TypeScript => "typed",
                            BlockLanguage::Tsx => "tsx",
                        }
                    )
                })
                .collect::<Vec<_>>()
                .join("|")
                .into_boxed_str(),
        ),
        Ask::Contract => {
            check(&EmbeddedSvelteAdapter, ".svelte");
            "held"
        }
        Ask::Aligned(text) => {
            if EmbeddedSvelteAdapter
                .blocks(Markup(text))
                .0
                .iter()
                .all(|block| {
                    text.get(block.start.0..block.start.0 + block.text.0.len())
                        == Some(block.text.0)
                })
            {
                "aligned"
            } else {
                "misaligned"
            }
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
