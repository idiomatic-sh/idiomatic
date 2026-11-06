# idiomatic-sh-ryo-language

The `Parse` and `Project` language contracts answer the portable relation catalog. Provider parsing, completion and model readiness remain language responsibilities.

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

### location

Indexed source byte offsets, physical lines, and columns shared by every language provider.

### `project_context`

The language domain's `project_context` concept: the host's search authority for resolving names within a project, and the candidate entries it observed.

### reasons

The language domain's `reasons` concept: what a language's tool or library said when a use case failed.

### semantics

The language domain's `semantics` concept: typed requests and outcomes for provider-owned semantic rendering.

### symbols

The language domain's `symbols` concept: the names of a file.

### targets

The language domain's `targets` concept: what a target a file writes (a link, an import, a url) names, as a path of the run.

### trees

The language domain's `trees` concept: a file's tree written from its library's events and the text between them, so that its tokens spell the whole file.

### words

The language domain's `words` concept: a value written as numbers, so it can leave a file and wait in the store.
