# idiomatic-sh-ryo-core

The core of ryo: the use cases that lint (and so run queries), open and describe, their domain rules and the ports they call.

## Domains

### admission

The core domain's `admission` concept: what typing does not prove of the output a language wrote for one file, so that a language's bug is refused where it is found and never reaches the store.

### `allocation_verdict`

The core domain's `allocation_verdict` concept: how many blocks the files of a lint allocated beyond what the parser library does on its own, and whether that is no more than the findings explain.

### answering

The core domain's `answering` concept: which rules have the answer of their cross-file steps from an earlier run, and what the answer of the others is made again from.

### budget

The core domain's `budget` concept: how much memory a run may use, and whether the repository and the process stay within it.

### choosing

The core domain's `choosing` concept: which rows of a file's findings are written.

### `corpus_copies`

The core domain's `corpus_copies` concept: the files of a corpus's inputs, once or many times over, under the paths a lint reads them by.

### crossing

The core domain's `crossing` concept: the cross-file steps of one rule, run on what the files handed over, and the findings they give.

### finish

The core domain's `finish` concept.

### handing

The core domain's `handing` concept: what a file handed the steps that read across files, as the store keeps it in its tables and gives it back.

### keeping

The core domain's `keeping` concept: what a run keeps of a repository for the next, and the digests that say whether it can be used.

### listing

The core domain's `listing` concept: the record of every file a run listed, whether a rule governed it or not.

### ordering

The core domain's `ordering` concept.

### project

The core domain's `project` concept: what the rules read of the project the manifests and the links between the files make, and whether it is made again or the earlier run's is used.

### reading

The core domain's `reading` concept: the steps of the visit of one file that need no port: what is asked of the file, how the rules run on its rows and what leaves it for the cross-file steps.

### recording

The core domain's `recording` concept: what a visit of a file comes to for the store the run keeps, and how what an earlier run kept of a file is used again.

### `relation_lookup`

The core domain's `relation_lookup` concept: the relation ryo owns that a name written in a query stands for.

### render

The core domain's `render` concept.

### reporting

The core domain's `reporting` concept.

### repository

The core domain's `repository` concept: the files of a run as it finds them, and what an earlier run kept of them.

### rules

The core domain's `rules` concept: the rules of a run as the domain reads them, and what is decided of them before any file is.

### storing

The core domain's `storing` concept: what a run keeps in its store for the next, what it can use again of what the last kept, and when it must read every file again.

### `value_cells`

The core domain's `value_cells` concept: the typed cell a value of the engine is in a row of the answer, and the text its file and text cells keep. No value becomes text for a reader here: only the command line writes cells as text.

### visit

The core domain's `visit` concept.

### workers

The core domain's `workers` concept: what each worker of a run owns while the files are visited, what every visit reads in common, and how what the workers made meets once they are done.
