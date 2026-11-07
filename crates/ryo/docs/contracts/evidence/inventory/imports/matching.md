# Imports: matching

Per concept: the concepts of its own crate it imports from (source-dependency edges, with the count of edges), the concepts and crate roots of other crates it imports from (static-import of a `ryo_*` path), and the concepts that import it.

| concept | imports (same crate) | imports (other crates) | imported by |
|---|---|---|---|
| domain/globbing | - | - | matching:inbound/use_cases/matching (13) |
| domain/matching | - | - | matching:inbound/use_cases/matching (10) |
| inbound/use_cases/matching | matching:domain/globbing (13); matching:domain/matching (10) | - | core:outbound/matching/adapters/globset (4); execution:domain/comparing (6); execution:domain/comparing/collections/comparisons (5); execution:domain/matching (12); operators:domain/replace (4); operators:domain/source_matches (5) |
