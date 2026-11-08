# Items: execution

One row per concept: every struct, enum, trait, function and type alias written with a visibility other than private, as `visibility kind name`.

| concept | items | items defined |
|---|---|---|
| application/publication | 3 | pub enum PublicationError; pub function publication; pub struct PublicationInput |
| application/rule_compilation | 13 | pub enum RuleCompilationError; pub(super) function compiled; pub(crate) function observed; pub(crate) function with_producers; pub(crate) struct CachedPlan; pub struct CompiledPlans; pub struct CompiledQueries; pub(crate) struct PlanCache; pub struct PreparedRuleIdentities; pub struct QueryCompilation; pub(crate) struct RuleCompilation; pub trait ExecutionCompilation; pub trait PreparedQueries |
| inbound/use_cases/execution | 2 | pub struct ExecutionUseCaseImpl; pub trait ExecutionUseCase |
| inbound/use_cases/metadata | 5 | pub struct AffectedFiles; pub struct CoverageFileCount; pub struct FileOrdinals; pub struct MetadataUseCaseImpl; pub trait MetadataUseCase |
| inbound/use_cases/prepare | 4 | pub enum PlanSource; pub struct PrepareRequest; pub struct PrepareUseCaseImpl; pub trait PrepareUseCase |
| inbound/use_cases/publication | 2 | pub struct PublicationUseCaseImpl; pub trait PublicationUseCase |
| outbound/matching | 2 | pub function globset; pub trait MatchingPort |
| outbound/matching/adapters/globset | 1 | pub(crate) struct MatchingGlobsetAdapter |
| outbound/query_language | 3 | pub enum QueryLanguageError; pub function use_case; pub trait QueryLanguagePort |
| outbound/query_language/adapters/use_case | 3 | pub(crate) struct CompileCalled; pub(crate) struct QueryLanguageUseCaseAdapter; pub(crate) struct TypecheckCalled |
