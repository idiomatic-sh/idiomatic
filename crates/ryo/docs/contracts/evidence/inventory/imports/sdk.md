# Imports: sdk

Per concept: the concepts of its own crate it imports from (source-dependency edges, with the count of edges), the concepts and crate roots of other crates it imports from (static-import of a `ryo_*` path), and the concepts that import it.

| concept | imports (same crate) | imports (other crates) | imported by |
|---|---|---|---|
| domain/capacity | core:outbound/scheduler (2) | core:outbound/scheduler (2) | - |
| domain/cell | relations:domain/relations (1); sdk:domain/file (1) | relations:domain/relations (1) | - |
| domain/file | - | - | sdk:domain/cell (1); sdk:domain/language (1); sdk:domain/project (1) |
| domain/host | - | - | - |
| domain/language | language:domain/files (2); language:domain/symbols (1); language:inbound/use_cases/parse (12); language:inbound/use_cases/project (1); relations:domain/relations (1); sdk:domain/file (1) | language:domain/files (1); language:domain/symbols (1); language:inbound/use_cases/parse (7); language:inbound/use_cases/project (1); relations:domain/relations (1) | - |
| domain/observation | relations:domain/relations (1); values:domain/observations (4); values:domain/plan_parts (1) | - | - |
| domain/operator | - | - | - |
| domain/parameter | - | - | - |
| domain/place | - | - | - |
| domain/project | sdk:domain/file (1) | - | - |
| domain/query | values:domain/program (6); values:domain/query_tokens (7) | values:domain/program (1); values:domain/query_tokens (3) | - |
| domain/relation | - | - | - |
