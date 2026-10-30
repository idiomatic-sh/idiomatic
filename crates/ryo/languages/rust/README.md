# idiomatic-sh-ryo-rust

Reads Rust source files and Cargo manifests for ryo: it implements `ryo::languages::Parse` and `ryo::languages::Project`. It answers 28 relations of the catalog (the 2 the engine gives every file are not a language's): 18 it reads from a file's tree, 5 (`directive`, `prose`, `mapping`, `entry`, `sequence_element`) are not in Rust, and 5 it reads from the project's manifests and the links between its files (`package_dependency`, `workspace_member`, `source_dependency`, `file_package`, `analyzed_source`); each answer is held to its fixtures by the engine's conformance test.

## Usage

The program that links the language asks for it by name, and the engine builds it over its ports:

```rust
let rust = ryo_rust::linked();
```

## Differences from the standard

The standard is rust-analyzer's parser, pinned in `tests/fixtures/standard/` (see its README): the Rust Reference defines no tree, so the
kinds are `SyntaxKind` of `ra_ap_parser` 0.0.347 and the tree is the one that parser prints.

- None in the tree of a file. For every input pinned there (the relations' inputs, the corpus and the suite, 460 files, valid and invalid
  alike), the language's tree printed as the parser's own tests print theirs is, line for line, the pinned one: every node and token
  of the same kind and text in the same place, white space and comments included. No exception is stated.
- The parser's three markers, `TOMBSTONE`, `EOF` and `__LAST`, which no tree it builds holds, are the extra `other`; a kind of the
  language is otherwise named as the parser names its kind, in kebab case.
- `ERROR` is listed among the tokens by the parser's `SyntaxKind` and is a node in the trees it builds, around the text it could not
  read; the language names its kind `error` among its tokens, and the element is a node (it has children) all the same. The
  comparison reads node or token from the element, as the parser's tree does.
- The `error` lines of a pinned dump are not compared as tree: the errors are the `syntax_error` relation's, one row for each message
  the parser reports.
- A macro call's arguments are also read again as expressions when the call is of a standard macro (`println!`, `vec!` and the like):
  those elements are the expansion's, marked as such under their call, and not in the standard's tree, which holds the call's token tree
  only. The comparison with the standard is made with no macro read again.
- The four suite files of other editions than 2024 are left out (`tests/fixtures/corpus/NOTICE`): the language reads one edition.

## Domains


### bindings

The rust domain's `bindings` concept.

### conditions

The rust domain's `conditions` concept: how Cargo shows the platform a dependency is for.

### declarations

The rust domain's `declarations` concept.

### dependencies

The rust domain's `dependencies` concept: what a package depends on by path, as Cargo's metadata says it.

### extras

The rust domain's `extras` concept: the kinds of its trees that rust-analyzer's parser has no name for.

### `fact_extraction`

The rust domain's `fact_extraction` concept: what a file says of its modules, read from the element table the parse has built and written into the notes.

### `fact_notes`

The rust domain's `fact_notes` concept: the facts of one file as the bytes the parse leaves for the project of the same run to read, and the readers and writers of the bytes the notes are made of.

### fields

The rust domain's `fields` concept: the role of every element under its parent.

### imports

The rust domain's `imports` concept.

### labels

The rust domain's `labels` concept: the roles the grammar gives by the kind of an element alone.

### language

The rust domain's `language` concept.

### `link_state`

The rust domain's `link_state` concept: what the language hands from its resolving of the project to the readers of the manifests' relations.

### links

The rust domain's `links` concept: how the source files of a project refer to one another, and the relations that say it, one item each.


### manifests

The rust domain's `manifests` concept: the relations the manifests of a project give, one item each.

### `module_tree`

The rust domain's `module_tree` concept: the module tree of a crate, drawn from the facts of its files, and the paths a file writes resolved against it.

### nodes

The rust domain's `nodes` concept: the nodes of its trees, named as rust-analyzer's parser names its kinds.

### `package_index`

The rust domain's `package_index` concept: the packages of a repository as extents of directories, and which package holds a file.

### packages

The rust domain's `packages` concept: what the `Cargo.toml` files of a repository declare, read as Cargo reads them.

### paths

The rust domain's `paths` concept: the places of a repository's files, written from its root with `/`.

### references

The rust domain's `references` concept.

### refinements

The rust domain's `refinements` concept.

### relations

The rust domain's `relations` concept.

### scopes

The rust domain's `scopes` concept.

### syntax

The rust domain's `syntax` concept.

### targets

The rust domain's `targets` concept: the source file of every target Cargo finds in a package, by what the manifest declares and by where the files are.

### tokens

The rust domain's `tokens` concept: the tokens of its trees, named as rust-analyzer's parser names its kinds.

### workspaces

The rust domain's `workspaces` concept: which workspace holds a package, and which packages a workspace holds.
