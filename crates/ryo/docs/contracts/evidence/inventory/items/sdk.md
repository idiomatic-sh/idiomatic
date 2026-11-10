# Items: sdk

One row per concept: every struct, enum, trait, function and type alias written with a visibility other than private, as `visibility kind name`.

| concept | items | items defined |
|---|---|---|
| domain/capacity | 2 | pub enum MemoryBudget; pub struct WorkerPool |
| domain/cell | 4 | pub enum Cell; pub enum Slot; pub struct HostNumber; pub struct HostText |
| domain/file | 4 | pub enum Unreadable; pub struct ReadFailure; pub struct RelativePath; pub struct Unread |
| domain/host | 5 | pub enum HostKind; pub struct HostColumn; pub struct HostColumnName; pub struct HostDeclaration; pub struct HostRelationName |
| domain/language | 5 | pub enum SourceLanguage; pub struct Language; pub struct LanguageBuild; pub struct LanguageName; pub struct Languages |
| domain/observation | 7 | pub enum ObservationKind; pub enum ObservationScope; pub struct BoundObservationInput; pub struct ObservationBindingName; pub struct ObservationColumn; pub struct ObservationColumnName; pub struct ObservationName |
| domain/operator | 3 | pub struct Example; pub struct OperatorName; pub struct Purpose |
| domain/parameter | 8 | pub enum FlagState; pub enum ParameterValue; pub struct ParameterCell; pub struct ParameterName; pub struct ParameterNumber; pub struct ParameterNumbers; pub struct ParameterTable; pub struct ParameterText |
| domain/place | 3 | pub enum Place; pub struct Column; pub struct Line |
| domain/project | 9 | pub enum ProjectDependencyState; pub enum ProjectEntryState; pub enum ProjectInventoryCompleteness; pub enum ProjectSearchScope; pub struct DependencyBytes; pub struct DependencyFingerprint; pub struct GivenProjectInventory; pub struct ProjectDependency; pub struct ProjectEntryObservation |
| domain/query | 10 | pub enum CatalogContent; pub enum QueryBundleError; pub enum ValidateError; pub struct ByteOffset; pub struct QueryModuleCatalog; pub struct QueryModuleSource; pub struct QueryOrigin; pub struct QuerySpan; pub struct QueryTraceSite; pub struct RyoQlQuery |
| domain/relation | 3 | pub struct ColumnName; pub struct Holds; pub struct RelationName |
