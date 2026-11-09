# Imports: matching

Per concept: the concepts of its own crate it imports from (source-dependency edges, with the count of edges), the concepts and crate roots of other crates it imports from (static-import of a `ryo_*` path), and the concepts that import it.

| concept | imports (same crate) | imports (other crates) | imported by |
|---|---|---|---|
| domain/globbing | - | - | matching:inbound/use_cases/matching (13) |
| domain/matching | - | - | matching:inbound/use_cases/matching (17) |
| inbound/use_cases/matching | matching:domain/globbing (13); matching:domain/matching (17) | - | core:inbound/use_cases/lint (4); core:inbound/use_cases/validate (1); execution:inbound/use_cases/compile_rules (1); execution:inbound/use_cases/metadata (4); execution:outbound/matching (3); execution:outbound/matching/adapters/globset (12); execution_domain:application/matching (21); execution_domain:domain/comparing (2); execution_domain:domain/comparing/collections/comparisons (12); execution_domain:domain/matching (33); execution_domain:domain/operator/contract (1); execution_domain:domain/table_scratch (2); execution_runtime:application/file_dag (3); execution_runtime:application/file_dag/program/facade_runtime/file_execution/presence (1); execution_runtime:application/file_dag/program/preparation (1); execution_store:application/file_dag/workspace (3); operators:domain/contract (8); operators:domain/evaluation (1); operators:domain/replace (2); operators:domain/source_matches (5); operators:inbound/use_cases/operators (9); operators:outbound/matching (11); operators:outbound/matching/adapters/use_case (12) |
