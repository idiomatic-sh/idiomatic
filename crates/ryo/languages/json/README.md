# idiomatic-sh-ryo-json

The JSON language of ryo: it reads `.json` and `.jsonc` files as JSON (RFC 8259) and as JSON with comments into syntax trees, and their objects, members, arrays and strings into ryo's keyed relations (`mapping`, `entry`, `sequence_element`, `string_literal`, `comment`). It reads with `jsonc_parser`, the parser of dprint and Deno, and is held to `JSONTestSuite` (every `y_` file read clean, every `n_` file reported) and to the tree RFC 8259's ABNF gives every file it pins in `tests/fixtures/standard`.

Which files are JSON with comments is decided by their path, as TypeScript and VS Code decide it: `.jsonc`, `.code-workspace`, `.babelrc`, `.eslintrc`, `.swcrc`, `tsconfig.json`, `jsconfig.json` and the files named like them (`tsconfig.build.json`, `jsconfig-base.json`). Every other `.json` file, and `.webmanifest`, is JSON as the standard defines it.

## Usage

Link the language with `ryo_json::linked()` and hand the result to the SDK's calls.

```rust
let language = ryo_json::linked();
assert_eq!(&*language.name().0, "json");
```

## Domains

### extras

The json domain's `extras` concept: the kinds of its trees that its standard has no name for.

### language

The json domain's `language` concept.

### nodes

The json domain's `nodes` concept: the nodes of its trees, named as its standard names them.

### relations

The json domain's `relations` concept: one reader for each relation the language reads.

### syntax

The json domain's `syntax` concept.

### tokens

The json domain's `tokens` concept: the tokens of its trees, named as its standard names them.

## Differences from the standard

The standard is RFC 8259, its ABNF pinned as `tests/fixtures/standard/json.abnf`, with the tree it gives each of the 302 files the language pins (the suite's valid and invalid files, the corpus, the relations' inputs and the allocation test's, recorded with the Python package `abnf` 2.9.0). The differences are these.

- **Separators are their punctuation alone.** The standard's `begin-object`, `end-object`, `begin-array`, `end-array`, `name-separator` and `value-separator` own the white space around them; the trees have it as `ws` tokens beside the punctuation, and a node ends at its last piece that is not white space (the standard's `end-object` takes the white space after a document's last value into the object; ours leaves it under the root). The test settles the standard's white space the same way before it compares.
- **Fewer nodes than rules.** Only the root (`json-text`), `object`, `member` and `array` are nodes; `value` is not in the tree, and the test takes the pieces of such a rule as the pieces of the rule it is in.
- **Comments are extras.** JSON with comments has `line-comment` and `block-comment`, which the standard lacks; a file read as JSON the standard defines gets a syntax error for each of them, and for each comma before a closing brace or bracket. The tree is the same whichever way the file is read.
- **A tree for what the standard refuses.** A file the library cannot read (every `n_` file of the suite that is not only a comment or a comma) is the tokens the library scans before it fails, flat under `json-text`, then the text it could not read as an `error` token, with the library's one syntax error; its objects and arrays are not nodes. A file read as JSON with comments that the standard refuses and the library reads is read clean.
- **Nesting to 512.** The library stops once more than 512 of its ranges are open, a member being one beside its object: arrays nest 513 deep and objects 256 deep, and what is deeper is a file it cannot read.
- **`null` is a value kind.** The shared kinds of a value gained `null` for this language (TOML never writes it): a member or an element whose value is `null` has an `entry` or `sequence_element` row of kind `null`, its text `null`.
- **Mappings.** The object that is the whole document is the root mapping, `standard`, with the root as its `id` and its owner; the value of a member is `inline` and owned by the member's name; an element of an array is a `sequence-member` owned by itself, its place in the array its ordinal.
- **Manifests.** The standard defines no project manifest, and the language answers every manifest relation as not in it (`src/inbound/contracts/project/contract.rs` cites the standard in each).
