# Items: operators

One row per concept: every struct, enum, trait, function and type alias written with a visibility other than private, as `visibility kind name`.

| concept | items | items defined |
|---|---|---|
| domain/aggregate | 1 | pub(crate) function aggregate |
| domain/append | 1 | pub function append |
| domain/capture | 5 | pub(crate) function matches; pub(crate) struct CaptureSlots; pub(crate) struct SlotPattern; pub(crate) struct SlotSubject; pub trait CaptureOutput |
| domain/contract | 5 | pub enum OperatorAnswer; pub enum OperatorCall; pub enum PortableOperatorFault; pub struct SemanticInterrupted; pub struct SourceByteLength |
| domain/equality_join | 10 | pub enum JoinInput; pub enum JoinRow; pub(crate) function join_row; pub(crate) function positions; pub struct JoinInterrupted; pub(crate) struct PositionSlots; pub(crate) struct PositionTarget; pub trait JoinCells; pub trait JoinDriver; pub trait PositionJoinDriver |
| domain/ordered_existence | 5 | pub enum PartitionExtremum; pub(crate) function flag; pub(crate) struct ExtremumWitness; pub struct OrderedExistenceScratch; pub trait OrderedExistenceDriver |
| domain/ordered_match | 2 | pub(crate) function pairs; pub trait OrderedMatchDriver |
| domain/project | 1 | pub function project |
| domain/rank | 1 | pub(crate) function rank |
| domain/reduction | 19 | pub enum AggregationCheckOrder; pub enum AggregationPlan; pub enum DigestDatum; pub(crate) enum Finishing; pub enum GroupColumns; pub(crate) enum GroupMembership; pub(crate) function check_program; pub(crate) function count_groups; pub(crate) function digest_groups; pub(crate) function finish; pub(crate) function hash_of; pub(crate) function push; pub struct AggregationHash; pub(crate) struct GroupCounts; pub(crate) struct HashScratch; pub(crate) struct ValueHash; pub trait DigestValues; pub trait GroupOutput; pub trait StreamingReduction |
| domain/replace | 3 | pub(crate) function replace_compiled; pub(crate) struct ReplacedText; pub(crate) struct ReplacementText |
| domain/select | 3 | pub(crate) enum PredicateOutcome; pub(crate) enum RowSelection; pub(crate) function select |
| domain/set_rows | 7 | pub enum SetInput; pub enum SetKind; pub(crate) function positions; pub(crate) function values; pub struct SetInterrupted; pub trait SetPositionsDriver; pub trait SetRowDriver |
| domain/source_matches | 7 | pub(crate) enum CarriageReturn; pub(crate) function occurrences; pub(crate) struct LineBytes; pub(crate) struct LineCursor; pub(crate) struct SourceBase; pub struct SourceMatchesInput; pub trait TextRowDriver |
| domain/span_between | 1 | pub function between |
| domain/span_bounds | 1 | pub function bounds |
| domain/words | 3 | pub(crate) function split; pub(crate) struct WordScratch; pub trait WordOutput |
| inbound/use_cases/operators | 2 | pub struct OperatorsUseCaseImpl; pub trait OperatorsUseCase |
