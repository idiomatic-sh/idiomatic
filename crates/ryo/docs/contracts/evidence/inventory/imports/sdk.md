# Imports: sdk

Per concept: the concepts of its own crate it imports from (source-dependency edges, with the count of edges), the concepts and crate roots of other crates it imports from (static-import of a `ryo_*` path), and the concepts that import it.

| concept | imports (same crate) | imports (other crates) | imported by |
|---|---|---|---|
| domain/capacity | - | core:outbound/scheduler (2) | - |
| domain/cell | sdk:domain/file (1) | relations:domain/relations (1) | - |
| domain/file | - | - | sdk:domain/cell (1); sdk:domain/language (1); sdk:domain/project (1) |
| domain/host | - | - | - |
| domain/language | sdk:domain/file (1) | language:domain/files (1); language:domain/symbols (1); language:inbound/use_cases/parse (7); language:inbound/use_cases/project (1); relations:domain/relations (1) | - |
| domain/observation | - | - | - |
| domain/operator | - | - | - |
| domain/parameter | - | - | - |
| domain/place | - | - | - |
| domain/project | sdk:domain/file (1) | - | - |
| domain/query | - | values:domain/program (1); values:domain/query_tokens (3) | - |
| domain/relation | - | - | - |
