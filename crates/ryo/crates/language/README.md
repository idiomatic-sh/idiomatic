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
