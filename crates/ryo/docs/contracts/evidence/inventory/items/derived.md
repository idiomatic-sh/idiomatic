# Items: derived

One row per concept: every struct, enum, trait, function and type alias written with a visibility other than private, as `visibility kind name`.

| concept | items | items defined |
|---|---|---|
| domain/contract | 44 | pub enum BuildOutcome; pub enum BulkRows; pub enum DerivedFault; pub enum DerivedInputs; pub enum DerivedView; pub enum Finding; pub enum Membership; pub enum MembershipSpace; pub enum MembershipView; pub enum NativeForm; pub enum On; pub enum OwnedOrderedKey; pub(crate) enum PayloadData; pub enum ViewRequest; pub function hash_key; pub function hash_row; pub function mix; pub function word_of; pub struct BuildWorkspace; pub struct ChainLink; pub struct ClosurePartition; pub struct CompactGraph; pub struct CycleGraph; pub struct DerivedPayload; pub struct ForestParents; pub struct HashWord; pub struct HashedRow; pub struct KeyCellsHash; pub struct KeyIndex; pub struct LiteralPostings; pub struct OrderedMatching; pub struct OwnedOrderedRow; pub struct PayloadFootprint; pub struct PreparedDerived; pub struct PreparedDfs; pub struct RawOrder; pub struct SharedBacking; pub struct SharedMarks; pub struct SharedPositions; pub struct WeakPositions; pub trait CanonicalTexts; pub trait PredicateRows; pub trait RowCells; pub trait ValueTexts |
| inbound/use_cases/derived | 2 | pub struct DerivedUseCaseImpl; pub trait DerivedUseCase |
