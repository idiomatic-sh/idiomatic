# idiomatic-sh-ryo-query_language

The RyoQL front end: the parser, the lowering and the plan a query compiles to; the language itself is described in `docs/ryoql.md` of the ryo product.

## Domains

### assembling

The `query_language` domain's `assembling` concept: a validated program lowered to a plan, by calling the concepts that lower each kind of step.

### calling

The `query_language` domain's `calling` concept: the calls that shape rows lowered to steps: `project`, `extend`, `join` and `antijoin`, and the dispatch of every call.

### capture_pushing

The `query_language` domain's `capture_pushing` concept: the filters made before the capture they follow, so the capture matches only the rows they let through.

### combining

The `query_language` domain's `combining` concept: the set calls (`union`, `intersection` and `difference`) of two inputs with the same columns, lowered to steps.

### comparing

The `query_language` domain's `comparing` concept: the sides of a comparison, resolved, and the test checked against their types.

### cutting

The `query_language` domain's `cutting` concept: the rows a file hands over, cut to the columns the steps after the files ask.

### fusing

The `query_language` domain's `fusing` concept: the filters and lookups of a plan that are read where they are found, instead of being made into tables first.

### idioms

The `query_language` domain's `idioms` concept: the pairs of functions with equal sets of texts, said as a comparison of digests.

### inlining

The `query_language` domain's `inlining` concept: the views of a relation read in place, and what an operation reads of them.

### joining

The `query_language` domain's `joining` concept: the join a `select` writes, and the columns of either side its `where` names.

### linking

Resolves fragment imports, input bindings and source identities before lowering.

### lowering

The `query_language` domain's `lowering` concept: the values, names and failures the concepts that lower a program share while a plan is built.

### observation_inputs

Validates bound input names and columns against the admitted catalog and hosted sources.

### planning

The `query_language` domain's `planning` concept: the steps added to the plan under construction, and where each one runs.

### pruning

The `query_language` domain's `pruning` concept: the steps nothing reads, dropped, and the inputs walked.

### pushing

The `query_language` domain's `pushing` concept: the lookups into a join of edges and elements made before the join.

### rendering

The `query_language` domain's `rendering` concept: the calls that read the physical text of the syntax tree, its rendering of a node and the width of a replacement.

### reshaping

The `query_language` domain's `reshaping` concept: the calls that count, digest, capture, split or rank rows, lowered to steps.

### resolving

The `query_language` domain's `resolving` concept: where the names a query writes become what they name, and where the arguments of a call are read.

### rewriting

The `query_language` domain's `rewriting` concept: the idiom of a plan that says in many rows what one aggregate says in few: the pairs of functions whose sets of texts are equal.

### selecting

The `query_language` domain's `selecting` concept: a `select` lowered to the join, the filter and the projection the query wrote.

### spanning

The `query_language` domain's `spanning` concept: the calls that compose the byte spans of rows and project their start and end.

### substituting

The `query_language` domain's `substituting` concept: validating and lowering literal regular-expression replacement, source occurrence matching and ordered request/reply matching, including their keys, ordering and output columns.

### syntax

The `query_language` domain's `syntax` concept.

### traversing

The `query_language` domain's `traversing` concept: the calls that walk an axis or close a relation, lowered to steps.

### validation

The `query_language` domain's `validation` concept.

### walking

The `query_language` domain's `walking` concept: the descents and neighbour lookups of the syntax tree made one walk.
