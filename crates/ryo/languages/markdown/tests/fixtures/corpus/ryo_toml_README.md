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

### keys

The toml domain's `keys` concept: how the pass knows a key of a document again.

### language

The toml domain's `language` concept.

### models

The toml domain's `models` concept: how a reader is answered from the model of its file.

### relations

The toml domain's `relations` concept: one reader for each relation the language reads.

### syntax

The toml domain's `syntax` concept.
