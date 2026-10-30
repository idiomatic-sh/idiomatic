# idiomatic-sh-ryo-toml

The TOML language of ryo: it reads `.toml` files as TOML 1.1, the version `toml_parser` and the `toml` crate parse, into syntax trees, and their tables, keys and values into ryo's keyed relations. It is held to toml-test as `toml-test-data` 2.14.1 publishes it, every file of its TOML 1.1 list, pinned in `tests/fixtures/suite`.

## Usage

Link the language with `ryo_toml::linked()` and hand the result to the SDK's calls.

```rust
let language = ryo_toml::linked();
assert_eq!(&*language.name().0, "toml");
```

## Domains

### documents

The toml domain's `documents` concept.

### extras

The toml domain's `extras` concept: the kinds of its trees that `toml.abnf` has no name for.

### keys

The toml domain's `keys` concept: how the pass knows a key of a document again.

### language

The toml domain's `language` concept.

### model

The toml domain's `model` concept: how a reader is answered from the model of its file.

### nodes

The toml domain's `nodes` concept: the nodes of its trees, named as `toml.abnf` names its rules.

### relations

The toml domain's `relations` concept: one reader for each relation the language reads.

### syntax

The toml domain's `syntax` concept.

### tokens

The toml domain's `tokens` concept: the tokens of its trees, named as `toml.abnf` names its terminal rules.

## Differences from the standard

The standard is `toml.abnf` of TOML 1.1.0, pinned in `tests/fixtures/standard`, which also holds the tree the ABNF gives each file of the corpus, the toml-test suite (valid and invalid), the relations' inputs and the allocation test's inputs (733 files, recorded with the Python package `abnf` 2.9.0). The kinds of the trees are its rules, and the adapter's tree of every one of those files is held to its dump. The differences are these.

- **A table holds its entries.** The standard's `std-table` and `array-table` are the header alone (`[a.b]`), and the entries below it are siblings under `toml`. The trees here put the entries until the next header inside the table's node, so that the table is what the header opens. The test projects our tree onto the standard's by putting a table's pieces after its header's closing bracket before it compares.
- **Fewer nodes than rules.** Only the rules that open a construct are nodes (`toml`, `std-table`, `array-table`, `keyval`, `key`, `array`, `inline-table`); the rest of the standard's nesting (`expression`, `table`, `val`, `simple-key`, `dotted-key`, `array-values`, `ws-comment-newline`...) is not in the tree, and the test takes the pieces of such a rule as the pieces of the rule it is in.
- **Separators are their punctuation alone.** The standard's `keyval-sep`, `dot-sep`, `array-sep`, `inline-table-sep` and the open and close rules of tables and arrays hold the white space, comments and newlines around the punctuation; the trees have those as their own `ws`, `comment` and `newline` tokens beside the separator, which the test does to the standard's tree too, joining a run of white space into one token.
- **Text it cannot read is in the tree.** The standard has no tree for a document that is not TOML (395 invalid files of toml-test are refused by the grammar); the trees hold the text the parser could not read as `error` tokens, and the language reports it as a syntax error. The test holds those files to this: the tree has an `error` token or the library says a diagnostic. The 99 invalid files of toml-test that the grammar accepts (a duplicate key, a table defined twice: invalid by the text of the standard and not by the grammar) have the grammar's tree, and the test holds them to it.
- **Arrays and inline tables nest to 80.** The parser is told to stop there, as the `toml` crate does, and what is deeper is text it could not read; the standard has no limit.
- **No stated exception.** Every one of the 733 pinned files agrees with the standard's tree after the projections above.
- **Manifests.** The standard defines no project manifest, and the language answers every manifest relation as not in it (`src/inbound/contracts/project/contract.rs` cites the standard in each).
