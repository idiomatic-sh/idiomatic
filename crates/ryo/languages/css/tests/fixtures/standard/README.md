# css standard

## Source

CSS Syntax Module Level 3 (https://www.w3.org/TR/css-syntax-3/), the standard the language's kinds are named by: its §4 Tokenization
lists the tokens (`domain/tokens`) and its §5.2 CSS Parsing Results the nodes (`domain/nodes`). The two sections are kept here as the
W3C publishes them, `tokenization.html` and `css-parsing-results.html` (`NOTICE`).

`<relation>.tree`, `corpus/<file>.tree` and `suite/<half>/<list>/<number>.tree` are the trees tinycss2 parses of `relations/<relation>/input.css`,
of each file of `corpus/` and of each file of `suite/`, at the same path with `.tree` for `.css`, recorded once and never edited: 37 files.
tinycss2 is the Python implementation of the algorithms of CSS Syntax Level 3 by the author of css-parsing-tests, the standard's own
conformance suite, and it is the tool that gives the whole tree of a file as the standard builds it (the suite only gives the expected
result of one small input at a time, which `suite/` holds the inputs of). A tree is printed, one line for each node and token, indented two
spaces under the node that holds it, by this recipe, with the interpreter and package of `Version`:

1. the file is decoded as UTF-8, a byte order mark dropped, as the standard's decode step does;
2. `tinycss2.parse_component_value_list(text, skip_comments=False)` gives the component values, and
   `tinycss2.parse_stylesheet(values, skip_comments=False, skip_whitespace=False)` the rules, so that nothing is skipped;
3. the content of a qualified rule is read with `tinycss2.parse_blocks_contents`, and so is the content of an at-rule whose name is
   one of the at-rules the specifications of CSS define a block of declarations and rules for (the list of the language's
   `RULE_BODIES`: `media`, `supports`, `container`, `layer`, `scope`, `font-face`, `page`, `keyframes` and the rest, without regard to
   case and to a vendor prefix), because the standard leaves to each at-rule the reading of its block; the content of any other is its
   component values;
4. a line is the type of the node (`qualified-rule`, `at-rule <name>`, `declaration <name> [important]`, `function <name>`, `block`
   for the block of a rule, `block (`, `block [` or `block {` for a simple block, `error <kind>`, `literal <text>`, and the token types
   `ident`, `string`, `url`, `number`, `percentage`, `dimension`, `hash`, `whitespace`, `comment`).

The language's tree of the same file, projected onto it (`adapter_test.rs`), must have the same nodes and tokens in the same order.

## Version

W3C Candidate Recommendation Draft, 1 October 2026: https://www.w3.org/TR/2026/CRD-css-syntax-3-20261001/

tinycss2 1.5.1 (https://pypi.org/project/tinycss2/, BSD 3-Clause), run with `uv run --with tinycss2==1.5.1 python`.
