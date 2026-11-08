# Imports: matching

Per concept: the concepts of its own crate it imports from (source-dependency edges, with the count of edges), the concepts and crate roots of other crates it imports from (static-import of a `ryo_*` path), and the concepts that import it.

| concept | imports (same crate) | imports (other crates) | imported by |
|---|---|---|---|
| domain/globbing | - | - | matching:inbound/use_cases/matching (13) |
| domain/matching | - | - | matching:inbound/use_cases/matching (10) |
| inbound/use_cases/matching | matching:domain/globbing (13); matching:domain/matching (10) | - | core:inbound/use_cases/lint (1); execution:outbound/matching (1); execution:outbound/matching/adapters/globset (4); execution_domain:domain/comparing (1); execution_domain:domain/comparing/collections/comparisons (4); execution_domain:domain/matching (8); execution_store:application/file_dag/workspace (1); operators:inbound/use_cases/operators (3); operators:outbound/matching (5); operators:outbound/matching/adapters/use_case (6) |
