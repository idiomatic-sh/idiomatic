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

### comparing

The core domain's `comparing` concept: whether the comparisons of a select hold for a row, decided by the family of each test and remembered by name and by text.

### `corpus_copies`

The core domain's `corpus_copies` concept: the files of a corpus's inputs, once or many times over, under the paths a lint reads them by.

### coverage

The core domain's `coverage` concept: which relations a file is read in part, and what an operator that trusts the absence of rows does about it.

### crossing

The core domain's `crossing` concept: the cross-file steps of one rule, run on what the files handed over, and the findings they give.

### describing

The core domain's `describing` concept: how the description of an operator and of its fields is made.

### features

The core domain's `features` concept: which features of the language a plan uses, and how many rows each was fed.

### finish

The core domain's `finish` concept.

### fragment

The core domain's `fragment` concept.

### frontiers

The core domain's `frontiers` concept: the rows a rule's cross-file steps are given as their inputs, from what the project wrote and from the words the files handed over.

### functions

The core domain's `functions` concept: the text a function of a comparison makes from the texts of its arguments.

### globbing

The core domain's `globbing` concept: whether a glob matches a text.

### handing

The core domain's `handing` concept: what a file handed the steps that read across files, as the store keeps it in its tables and gives it back.

### hosted

What the `lint` use case makes of the relations the host declares: what the compiler is told, the lines of a configuration, and the tables the rows make.

### joins

The core domain's `joins` concept: the lowered steps that combine two steps: an antijoin, a union and a join.

### keeping

The core domain's `keeping` concept: what a run keeps of a repository for the next, and the digests that say whether it can be used.

### `kind_check`

The core domain's `kind_check` concept: the words a rule writes for a kind of node or token, checked against the kinds the linked languages have.

### `kind_sets`

The core domain's `kind_sets` concept: the comparisons of a select that name a kind, decided once for each name as a set of kinds.

### layout

The core domain's `layout` concept: how a plan is laid out for the worker that runs it, which steps keep their rows and what columns each answers.

### listing

The core domain's `listing` concept: the record of every file a run listed, whether a rule governed it or not.

### lowering

The core domain's `lowering` concept: which steps of a plan read only the tree of a file, and what each is when it runs over the positions of the elements.

### matching

The core domain's `matching` concept: cached regular-expression matches and literal text replacement over typed table cells.

### operands

The core domain's `operands` concept: what one side of a comparison is for one row, borrowed from the row, the bound parameters or the plan.

### operating

The core domain's `operating` concept: what an operator reads, writes and fails with.

### operator

The core domain's `operator` concept.

### ordering

The core domain's `ordering` concept.

### pipeline

The core domain's `pipeline` concept: the per-file steps of a plan run as one pass over the rows.

### placement

The core domain's `placement` concept.

### positions

The core domain's `positions` concept: the rows of a lowered step, which are the positions of the elements of a file, and what is read from them.

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

### replay

The core domain's `replay` concept: how the rows a step makes are passed on, one by one, to what takes them.

### reporting

The core domain's `reporting` concept.

### repository

The core domain's `repository` concept: the files of a run as it finds them, and what an earlier run kept of them.

### rules

The core domain's `rules` concept: the rules of a run as the domain reads them, and what is decided of them before any file is.

### selections

The core domain's `selections` concept: the lowered steps that keep rows: a select of the tree or of another step, and a projection.

### `shared_walk`

The core domain's `shared_walk` concept: the `select` steps of the call's rules over the elements of a file, merged at run time so that the elements are visited once, each handed to every step that reads its kind, and a text is decided once for all of them.

### spelling

The core domain's `spelling` concept: whether names that share a hash are spelled the same.

### `step_classes`

The core domain's `step_classes` concept: the `select` steps of the call's rules over the elements of a file, sorted into the classes of the steps that select the same elements, so that the shared walk decides each class once.

### storing

The core domain's `storing` concept: what a run keeps in its store for the next, what it can use again of what the last kept, and when it must read every file again.

### `table_cells`

The core domain's `table_cells` concept.

### `table_dedup`

The core domain's `table_dedup` concept.

### `table_groups`

The core domain's `table_groups` concept.

### `table_model`

The core domain's `table_model` concept.

### `table_reach`

The core domain's `table_reach` concept.

### `table_scratch`

The core domain's `table_scratch` concept.

### `table_texts`

The core domain's `table_texts` concept.

### `table_words`

The core domain's `table_words` concept.

### `text_gate`

The core domain's `text_gate` concept: the comparisons of a select that ask of the text of an element something fixed, decided once for each text of the file, and the columns the comparisons read.

### `tree_join`

The core domain's `tree_join` concept: how a join finds the rows of a tree for the rows of its left input, one at a time, and what it keeps while it does.

### `tree_walk`

The core domain's `tree_walk` concept: walking the parent and child hierarchy of a file from the start rows of an `axis`, with what the walk takes from the scratch while it runs.

### trust

The core domain's `trust` concept: which reads of the relations a plan trusts the absence of rows of, and whether a relation read in part could change what they make.

### `value_cells`

The core domain's `value_cells` concept: the typed cell a value of the engine is in a row of the answer, and the text its file and text cells keep. No value becomes text for a reader here: only the command line writes cells as text.

### visit

The core domain's `visit` concept.

### walks

The core domain's `walks` concept: the lowered step that walks the tree from the rows of another step, and how the elements it reaches are told to be answers.

### workers

The core domain's `workers` concept: what each worker of a run owns while the files are visited, what every visit reads in common, and how what the workers made meets once they are done.
