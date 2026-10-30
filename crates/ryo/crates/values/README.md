# idiomatic-sh-ryo-values

The value objects ryo's crates name across their boundaries: the files of a run, the host's relations, the plan a query compiles to, what a run keeps of each file, and what ryo says of what it owns. Plain data and the pure functions over it; no port, no use case.

## Domains

### descriptions

The values domain's `descriptions` concept: what ryo says of the relations and the operators it owns.

### files

The values domain's `files` concept: the files of a run and the globs that name them.

### hosted

The values domain's `hosted` concept: the relations a host declares to a run, and their rows.

### `plan_parts`

The values domain's `plan_parts` concept: the numbers, text and comparisons a query plan is made of.

### plans

The values domain's `plans` concept: a compiled query, its steps and what each reads.

### program

The values domain's `program` concept: the shape of RyoQL text once it is read.

### records

The values domain's `records` concept: what a run kept of each file, and of the repository, and the typed `Cell` a row of the answer holds: a file, a text, a number or a span, never turned into text before the command line prints it.

### runs

The values domain's `runs` concept: what a run is asked, and what it keeps of a repository for the next.

### `stored_facts`

The values domain's `stored_facts` concept: what a run keeps of the words and the text of a file.

### stores

The values domain's `stores` concept: what a lint keeps for the next run, as the tables of its store.
