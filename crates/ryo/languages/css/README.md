# idiomatic-sh-ryo-css

The CSS language of ryo: it reads `.css` files with `cssparser`, Servo's CSS Syntax Level 3 parser, into syntax trees in the component model of CSS Syntax Level 3 (its nodes and tokens, named as it names them), and their comments, strings, `@import`s and `url()`s into ryo's relations. It is held to css-parsing-tests as rust-cssparser 0.38.0 vendors them, pinned in `tests/fixtures/suite`.

## Usage

Link the language with `ryo_css::linked()` and hand the result to the SDK's calls.

```rust
let language = ryo_css::linked();
assert_eq!(&*language.name().0, "css");
```

## Domains

### extras

The css domain's `extras` concept: what a tree keeps so that its tokens spell the file, which the standard does not name.

### language

The css domain's `language` concept: which files are CSS.

### nodes

The css domain's `nodes` concept: the parsing results of CSS Syntax Level 3, the standard the nodes are named by.

### relations

The css domain's `relations` concept: one reader for each relation the language reads.

### syntax

The css domain's `syntax` concept: the kinds of a stylesheet's tree (a node, a token or an extra) and how to move about it.

### targets

The css domain's `targets` concept: what an `@import` or a `url()` points at, and what it belongs to.

### tokens

The css domain's `tokens` concept: the tokens of CSS Syntax Level 3's tokenizer, the standard the tokens are named by.

## Differences from the standard

The tree of every file pinned in `tests/fixtures/standard` (37: the relations' inputs, the corpus and the suite) is compared with the tree
tinycss2 prints for it, in the nodes and tokens of CSS Syntax Level 3 (`adapter_test.rs`). 36 agree. What differs, and what the comparison
does about it:

| Difference | Reason | In the comparison |
|---|---|---|
| The tree keeps every byte of the file as a token: white space, comments (the standard's tokenizer outputs none), a leading byte order mark, the `;` and `}` that end a rule, the closing `)`, `]` or `}` of a function or block, and the `<!--` and `-->` between rules | the tokens spell the file, which the standard's tree does not need | the projection drops them from our tree and the print has none |
| `!important` is the delim-token `!` and the ident-token `important` of the declaration | the standard's tokenizer gives those two tokens; tinycss2 turns them into a flag | the projection reads them as one `important`; in `syntax_child` the ident-token is in the role `important` under its declaration, the declaration's name in `name` and each component value of its value in `value` |
| A qualified rule with no block (`foo 4`) is a `qualified-rule` holding its tokens, with a diagnostic | the library reports the rule as an error and the tokens stay in the file | the projection reads it as the `error` the print has |
| A first rule `@charset` has no node: its tokens lie in the stylesheet | the library's stylesheet parser skips it without calling the adapter; the standard (§8.3) parses it as an unrecognized rule and drops it at grammar-checking | the comparison leaves that rule out of both trees |
| An `@charset` that is not the first rule has no node either (stated exception: `suite/invalid/stylesheet/05.css`) | the library refuses the rule, recovers from it and keeps its tokens as the stylesheet's | none: the file is the one that differs |
| The block of an at-rule holds declarations and rules only for the at-rules of `RULE_BODIES` and component values for any other | the standard leaves the reading of a block to each at-rule's own specification | the print is made with the same list |
| A declaration or rule the library recovers from inside a block keeps its tokens as those of the rule that holds it, with no `error` node | the library reports it with a diagnostic over the text it skipped | not compared: no pinned file has one |
| The language reads a file as UTF-8 and the bytes from the first that are not are one error token with a diagnostic | the standard decodes any bytes, replacing the invalid ones | not compared: every pinned file is UTF-8 |
| A block nested deeper than the library reads (its limit) is one error token | the standard has no limit | not compared: no pinned file nests so deep |
