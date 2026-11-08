# idiomatic-sh-ryo-values

The value objects ryo's crates name across their boundaries: the files of a run, the host's relations, the plan a query compiles to, what a run keeps of each file, and what ryo says of what it owns. Plain data and the pure functions over it; no port, no use case.

## Domains

### `compact_table`

The values domain's `compact_table` concept: a table of packed words read in place, never decoded whole.

### descriptions

The values domain's `descriptions` concept: what ryo says of the relations and the operators it owns.

### files

The values domain's `files` concept: the files of a run and the globs that name them.

### hosted

The values domain's `hosted` concept: the relations a host declares to a run, and their rows.

### lineage

Which file the rows of a step come from and which of its columns lie in that file: the one definition the planner's rewrites and the engine both read.

### observations

The values domain's `observations` concept: Explicit authored outputs and the typed dependencies between rules, and the readiness a run records for them.

### `plan_parts`

The values domain's `plan_parts` concept: the numbers, text and comparisons a query plan is made of.

### plans

The values domain's `plans` concept: a compiled query, its steps and what each reads.

### program

The values domain's `program` concept: the shape of RyoQL text once it is read.

### `query_bindings`

The values domain's `query_bindings` concept: The immutable bindings a host hands a query.

### `query_operators`

The values domain's `query_operators` concept: the operators of the engine, the kinds of operation a plan holds and the output choices of their steps.

### `query_syntax`

The values domain's `query_syntax` concept: the expressions, conditions and sources a RyoQL step is written with.

### `query_tokens`

The values domain's `query_tokens` concept: the spans, names and sources RyoQL text is written from, and where a node came from.

### records

The values domain's `records` concept: what a run kept of each file, and of the repository, and the typed `Cell` a row of the answer holds: a file, a text, a number or a span, never turned into text before the command line prints it.

### `row_texts`

The values domain's `row_texts` concept: the text of rows, where each rule, line, row and column of a finding is placed in it, and the totals of rows.

### rows

The values domain's `rows` concept: The neutral row carriers the engine and the semantic capabilities share.

### `rule_requests`

The values domain's `rule_requests` concept: The portable declarations of a rule request, and the refusals preparing or running one can end in.

### `run_metadata`

The values domain's `run_metadata` concept: The facts a run keeps of a compiled rule once its executable plan is consumed.

### runs

The values domain's `runs` concept: what a run is asked, and what it keeps of a repository for the next.

### `stored_facts`

The values domain's `stored_facts` concept: what a run keeps of the words and the text of a file.

### `stored_findings`

The values domain's `stored_findings` concept: what a store keeps of a finding: where it was made, its values and the text it names.

### stores

The values domain's `stores` concept: what a lint keeps for the next run, as the tables of its store.

### `table_words`

The values domain's `table_words` concept: The words a table is encoded as, and the compact table read back from them.
