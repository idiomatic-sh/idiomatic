# Items: derived

One row per concept: every struct, enum, trait, function and type alias written with a visibility other than private, as `visibility kind name`.

| concept | items | items defined |
|---|---|---|
| domain/adjacency | 3 | pub(crate) function build_borrowed; pub struct CompactGraph; pub struct ForestParents |
| domain/allocation | 3 | pub struct AllocationBytes; pub struct AllocationKey; pub struct ElementCapacity |
| domain/closure_adjacency | 4 | pub(crate) function build; pub function capacities; pub struct ClosureCapacities; pub struct ClosurePartition |
| domain/contract | 7 | pub enum BuildInputs; pub enum BuildOutcome; pub enum DerivedView; pub(crate) enum PayloadData; pub struct BuildWorkspace; pub struct DerivedPayload; pub struct PreparedDerived |
| domain/cycle_adjacency | 4 | pub(crate) function build; pub struct CycleGraph; pub struct LeavingNodes; pub struct LeavingStarts |
| domain/dfs | 5 | pub(crate) function build; pub(crate) struct DfsColumns; pub struct EdgeRun; pub struct EdgeSlot; pub struct PreparedDfs |
| domain/dispatch | 3 | pub function build; pub function native_finding; pub function view |
| domain/equality | 4 | pub(crate) function build; pub function capacities; pub struct KeyIndex; pub struct KeyIndexCapacities |
| domain/fault | 2 | pub enum DerivedFault; pub struct InputSlot |
| domain/footprint | 7 | pub function describe; pub function working_bytes; pub struct BackingKind; pub struct PartitionEntry; pub struct PayloadFootprint; pub struct SharedBacking; pub struct VertexEntry |
| domain/key | 9 | pub enum On; pub function hash_key; pub function hash_row; pub function mix; pub function word_of; pub struct ChainLink; pub struct HashWord; pub struct HashedRow; pub struct KeyCellsHash |
| domain/membership | 5 | pub(crate) enum DerivedMembership; pub enum MarkSpace; pub enum MembershipView; pub(crate) function build; pub struct MarkCount |
| domain/native | 3 | pub enum NativeFinding; pub enum NativeForm; pub(crate) function finding |
| domain/ordered | 5 | pub enum OwnedOrderedKey; pub(crate) function build; pub struct OrderedMatching; pub struct OrderedText; pub struct OwnedOrderedRow |
| domain/partition | 2 | pub(crate) enum KeyRelation; pub(crate) function ordered_positions |
| domain/positions | 1 | pub struct DenseMarks |
| domain/preparation | 2 | pub function kind; pub function prepare |
| domain/request | 1 | pub enum DerivedRequest |
| domain/row_cells | 4 | pub enum BulkRows; pub trait CanonicalTexts; pub trait RowCells; pub trait ValueTexts |
| domain/sharing | 5 | pub(crate) function marks_capacity; pub(crate) function marks_key; pub struct SharedGraph; pub struct SharedKeyIndex; pub struct SharedMarks |
| domain/vertices | 9 | pub enum EdgeDirection; pub struct NeighborList; pub struct OffsetList; pub(crate) struct ParentList; pub(crate) struct ParentSlice; pub struct PartitionId; pub struct PartitionList; pub struct VertexId; pub struct VertexKey |
| inbound/use_cases/build | 2 | pub struct BuildUseCaseImpl; pub trait BuildUseCase |
| inbound/use_cases/prepare_derived | 2 | pub struct PrepareDerivedUseCaseImpl; pub trait PrepareDerivedUseCase |
| inbound/use_cases/view | 2 | pub struct ViewUseCaseImpl; pub trait ViewUseCase |
