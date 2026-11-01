# idiomatic-sh-ryo-language

The relations ryo owns, and the two traits a language implements to fulfil them: `Parse` and `Project`, one required method for each relation of the catalog.

## Domains

### answers

The language domain's `answers` concept: how a language answers a relation of the catalog: the reader it gives, or why it gives none.

### completion

The language domain's `completion` concept: the finishing of the table of a file once it is built.

### elements

The language domain's `elements` concept: the element table of a file.

### expansions

The language domain's `expansions` concept: the trees a language reads under a macro call, and the marks on the elements they make.

### files

The language domain's `files` concept: the path every part of the engine names a file of the repository by.

### `kind_index`

The language domain's `kind_index` concept: the positions of a table grouped by kind.

### kinds

The language domain's `kinds` concept: how a language's kinds of node and token are numbered in the element table, named and listed, made from its own vocabularies.

### location

Indexed source byte offsets, physical lines, and columns shared by every language provider.

### reasons

The language domain's `reasons` concept: what a language's tool or library said when a use case failed.

### relations

The language domain's `relations` concept: the catalog of the relations ryo owns, typed.

### symbols

The language domain's `symbols` concept: the names of a file.

### targets

The language domain's `targets` concept: what a target a file writes (a link, an import, a url) names, as a path of the run.

### terms

The language domain's `terms` concept: the closed sets of words every language answers in (what a declaration is, how a parameter is taken, what a value is), as enums.

### trees

The language domain's `trees` concept: a file's tree written from its library's events and the text between them, so that its tokens spell the whole file.

### words

The language domain's `words` concept: a value written as numbers, so it can leave a file and wait in the store.

## Semantic syntax projections

`semantic-syntax` and `semantic-child` are explicit projections, separate from
native lossless syntax. Providers without embedded expressions reuse the shared
native projection readers. A provider that parses an embedded expression keeps
its logical token text as owned derived symbols and records its physical source
mapping independently. `embedded-expression` identifies the carrier, root,
evaluation context, and mapping fidelity. An embedded occurrence is distinct
from a macro expansion and from parser-implied syntax.

Every provider inherits conformance checks for native row and field-predicate
preservation when there are no embeddings, physical span bounds, source-coordinate
identity, and deterministic retained-table reuse. Field predicates also participate
in core demand planning: requesting a semantic edge field must request its roles,
even when that field is filtered out of the final query result.

### Advertised optional capabilities

Every parser explicitly declares its `OptionalCapability` set. The catalog currently includes
canonical identifiers, qualified-name provenance, embedded expressions, expression shapes,
static binding events/reads/uses/shadows, implicit symbols, literal properties, interpolated-
string fragments, language targets, constant keys, and percent-format parse/field facts. Every
other catalog relation remains the required baseline, including both semantic syntax
projections. Advertising a capability promises a reader; it cannot return `NotYetRead`.
Duplicate declarations fail conformance.

An unadvertised optional reader is never invoked. Its absence is `NotYetRead`, and a direct
read records missing rows rather than a known-empty relation. The conformance run prints
all unavailable capabilities even when the provider otherwise passes. Retained-reader
checks exercise unavailable → advertised → unavailable requests; native projections,
field filtering, source mapping, and reparse checks remain shared requirements.

Current availability is explicit:

| Providers | Optional promises |
| --- | --- |
| Python | Its declared semantic, literal, binding, target, constant-key, and percent-format readers |
| JSON, TOML | Identifier and qualified-name readers |
| Markdown | Qualified-name reader |
| CSS, Go, HTML, JavaScript, Rust, TypeScript | None; these optional facts remain unavailable |

Percent-format parsing and conversion-field facts are currently advertised only by Python;
other providers explicitly report `NotYetRead` for those two relations.

The parser-implied nodes of languages such as HTML retain their explicit absent source
span in the semantic projection. Shared guards accept that only for an actual implied
node, require no fabricated source-location row, and still reject unlocated ordinary nodes.
