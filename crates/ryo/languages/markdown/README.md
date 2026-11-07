# idiomatic-sh-ryo-markdown

The Markdown language of ryo: it reads `.md` and `.markdown` files as `CommonMark` with GitHub's extensions (tables, footnotes, strikethrough, task lists, alerts, math) and front matter, the way `pulldown-cmark` parses them, into lossless syntax trees, and their HTML comments and code into ryo's `comment` and `string_literal` relations. It is held to the `CommonMark` 0.31.2 spec examples, pinned in `tests/fixtures/suite`.

## Usage

Link the language with `ryo_markdown::linked()` and hand the result to the SDK's calls.

```rust
let language = ryo_markdown::linked();
assert_eq!(&*language.name().0, "markdown");
```

## Domains

### extras

The markdown domain's `extras` concept: the kinds of its trees that mdast and micromark have no name for.

### labels

The markdown domain's `labels` concept: the names a document gives its places in raw HTML, which a link can point at.

### language

The markdown domain's `language` concept: which files are Markdown.

### model

The markdown domain's `model` concept: the sections a file's headings open, which nest as the headings' levels do.

### nodes

The markdown domain's `nodes` concept: the nodes of its trees, named as mdast names its types.

### prose

The markdown domain's `prose` concept: the text a reader reads in a run of a document.

### relations

The markdown domain's `relations` concept: one reader for each relation the language reads.

### syntax

The markdown domain's `syntax` concept: the kinds of its trees and how a reader moves about them.

### targets

The markdown domain's `targets` concept: what a link destination names, as the path of a file of the run.

### tokens

The markdown domain's `tokens` concept: the tokens of its trees, named as micromark names its token types.

## Differences from the standard

The standard is mdast, as `mdast-util-from-markdown` builds it with the micromark extensions that match the adapter's options (`tests/fixtures/standard/README.md` names the tool and versions). Every file pinned there (12 of `corpus/`, the 652 `CommonMark` examples of `suite/valid/` and 8 relation inputs) has mdast's node types in mdast's nesting once our tree is projected onto mdast, but four. The projection is `adapter_test.rs`'s; what it does is a difference of the tree, and the four files are differences of the library.

The four files:

- `corpus/ruff_CONTRIBUTING.md`: the line `[!NOTE]` of a GitHub alert is a paragraph of text in mdast; `pulldown-cmark` reads it as the marker of the quote, which is markup in our tree.
- `suite/valid/link-reference-definitions/example-204.md` and `suite/valid/links/example-544.md`: a label defined twice is two `definition` nodes in mdast; the library lists the first definition of a label only, so the second is markup in our tree.
- `suite/valid/setext-headings/example-098.md`: `---` over `---` is empty front matter (`yaml`) in mdast; the library reads two thematic breaks.

The projection, where our tree is not mdast's as it stands:

- Our trees are lossless: the markers, the fences, the destinations of links, the white space and the line ends are tokens (`markup`, `whitespace`, `line-ending`), which mdast has no nodes for. A soft break is the token `line-ending`; mdast joins it into the `text` around it, and joins a run of `text` that the library splits at an entity or an escape.
- The lines of a tight list item, and of a footnote definition, are children of the item; mdast puts them in a `paragraph`.
- The alternative text of an image is the children of the `image`; mdast has it as the field `alt`. The text of front matter is a child of `yaml` and `toml`; mdast has it as `value`.
- The task marks of a list item are the extras `task-checked` and `task-unchecked`; mdast has the field `checked`. The depth of a heading is carried on its marker, as mdast's field `depth`.
- A heading opens a `section`, which mdast has no node for: it holds the heading and the blocks after it, among the same blocks, up to the next heading as deep or shallower, nested as the headings are, and ends where its last block ends; a block's parent is its section. The projection gives the blocks of a section to what holds it. `scope`, `defines` and `prose` still name a section by its heading.
- The kinds mdast and micromark have no name for are extras: `markup`, `yaml`, `toml`, `math`, `inline-math`, the task marks, `section` and `error`.

What the language does not read as the standard does:

- Math is not mdast's in one case no pinned file holds: `pulldown-cmark` gives `$$x$$` inside a line, and `$$` on lines of their own, as display math in a paragraph (extra `math`); `mdast-util-math` has `inlineMath` for the first and a `math` node beside the paragraphs for the second.
- Bare URLs are not links: the GFM autolink literal is an extension `pulldown-cmark` does not have, and the pinned trees are printed without it.
- Heading attributes, definition lists, superscript, subscript and wiki links are off, so their syntax is text, as GitHub shows it.
- Bytes that are not UTF-8: the library reads the text up to the first such byte and the rest is the extra `error` with a diagnostic; mdast-util-from-markdown reads text and has no tree for such a file, so `relations/syntax_error` has no pinned tree.
- The language has no function, call, import, assignment, parameter, annotation, decorator, inheritance, mention, mapping, entry or sequence element, and no project manifest: mdast defines none of them, and each answer cites it (`src/inbound/contracts`).
