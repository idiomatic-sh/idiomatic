# idiomatic-sh-ryo-execution-domain

Holds the execution engine's domain, the bottom of its crates: the compiled program, the operator contracts and drivers, the tables, the shared walk and the pure semantics the engine crates above it share; no port, no use case, and only the engine crates depend on it.

## Domains

### completion

Decides physical completeness independent of execution scheduling.

### contributor_rows

Keeps only the rows of an absence-based answer that the missing rows cannot change.

### coverage

Tracks which source relations and file partitions are complete or partial.

### derived_views

Defines pure semantics for views prepared and acquired by the owner.

### describing

Builds operator and field descriptions for request reporting.

### executable_program

Merges a request's rules into one logical program and assembles the planned program the file pass runs.

### features

Attributes observed row work to the features used by a request.

### finish

Normalizes producer names and prepares portable publication values.

### frontier_capture

Describes the frontier a rule retains and the scope it is captured in.

### frontier_coverage

Carries a coverage of files to the file-frontier boundary and restores it.

### hosted

Describes host-provided relations and their declared row shapes.

### kind_check

Checks query kind names against the linked language vocabulary.

### kind_sets

Prepares named kind comparisons as sets of admitted kinds.

### observation_schedule

Schedules the rules that read named observations and links the plans beneath them.

### operating

Defines operator inputs, outputs and refusal semantics.

### operation_schema

Infers and checks the ordered columns produced by each operation.

### operator

Holds prepared operator metadata and borrowed semantic contracts.

### output_coverage

Derives the completeness of an operator's answer from its operands.

### placement

Decides which configured rules govern a source file.

### plan_expansion

Rewrites the operations this engine never runs into the operations it does, one canonical shape each, before a plan reaches it.

### prepared_request

Holds immutable request declarations consumed by compilation.

### row_emission

Preserves physical uncertainty when rows are delivered.

### rule_intent

Freezes each rule's emit, observation and frontier demand before any file is walked.

### rules

Looks up admitted rule coordinates in the compiled request.

### source_partitions

Declares the immutable producer partitions of a compiled request.

### spelling

Checks equal name hashes against retained text to detect collisions.

### table_model

Defines table representations and their physical row access.

### tree_walk

Traverses parent and child relations for axis operations.

### trust

Tracks relation reads that rely on the absence of missing rows.
