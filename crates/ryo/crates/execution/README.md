# idiomatic-sh-ryo-execution

Owns Ryo's compiled request execution, source admission, shared scheduling, publication and release through inbound use cases.

## Domains

### column_kernel

Defines the column-oriented join kernels used by admitted execution.

### comparing

Evaluates typed selection comparisons against borrowed rows and bindings.

### coverage

Tracks which source relations and file partitions are complete or partial.

### crossing

Carries publication data and readiness across file and project phases.

### derived_views

Defines pure semantics for views prepared and acquired by the owner.

### describing

Builds operator and field descriptions for request reporting.

### executable_program

Defines immutable compiled regions, producers and dependency edges.

### features

Attributes observed row work to the features used by a request.

### file_program

Compiles per-file operation dependencies and admissible cut points.

### finish

Normalizes producer names and prepares portable publication values.

### frontiers

Describes the retained rows and project facts read by cross-file steps.

### functions

Computes comparison function text from typed operand values.

### hosted

Describes host-provided relations and their declared row shapes.

### kind_check

Checks query kind names against the linked language vocabulary.

### kind_sets

Prepares named kind comparisons as sets of admitted kinds.

### lowering

Derives tree-position shapes for eligible file-local operations.

### matching

Evaluates compiled text patterns over borrowed query values.

### native_positions

Evaluates admitted select, project, join, antijoin, union and axis positions.

### numeric_values

Orders canonical decimal and binary numeric values consistently.

### operands

Reads typed comparison operands from rows, parameters and literals.

### operating

Defines operator inputs, outputs and refusal semantics.

### operation_schema

Infers and checks the ordered columns produced by each operation.

### operator

Holds prepared operator metadata and borrowed semantic contracts.

### ordered_existence

Checks ordered existence against prepared join inputs.

### placement

Decides which configured rules govern a source file.

### positions

Represents borrowed tree coordinates and their row geometry.

### prepared_request

Holds immutable request declarations consumed by compilation.

### replay

Describes how produced rows are passed to their consumers.

### rules

Looks up admitted rule coordinates in the compiled request.

### selections

Evaluates tree selections and projections over borrowed rows.

### shared_walk

Groups compatible selections for one request-wide file walk.

### spelling

Checks equal name hashes against retained text to detect collisions.

### step_classes

Classifies compatible selection steps before shared traversal.

### stored_graph

Describes generation-bound graph population and adjacency.

### stored_selection

Describes selectors prepared once for stored relations.

### table_cells

Reads and validates typed cells in produced tables.

### table_dedup

Tracks distinct table rows while preserving encounter order.

### table_groups

Groups table rows for aggregate operations.

### table_model

Defines table representations and their physical row access.

### table_reach

Traverses reachable table nodes in source order.

### table_scratch

Holds per-invocation table workspace and work counters.

### table_texts

Resolves table text cells through their retained name sources.

### text_gate

Prepares fixed text comparisons for tree selection.

### tree_join

Matches tree edges to admitted left rows in demand order.

### tree_walk

Traverses parent and child relations for axis operations.

### trust

Tracks relation reads that rely on the absence of missing rows.

### walks

Applies filtered tree walks to admitted position inputs.
