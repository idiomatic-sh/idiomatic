# Items: operators

One row per concept: every struct, enum, trait, function and type alias written with a visibility other than private, as `visibility kind name`.

| concept | items | items defined |
|---|---|---|
| domain/aggregate | 3 | pub(crate) enum Aggregation; pub(crate) function aggregate; pub(crate) function evaluate |
| domain/append | 2 | pub function append; pub(crate) function evaluate |
| domain/axis | 5 | pub enum AxisAdmission; pub enum AxisCandidates; pub(crate) function evaluate; pub trait AxisDriver; pub trait AxisEvaluation |
| domain/capture | 8 | pub(crate) function evaluate; pub(crate) function matches; pub struct CaptureSlots; pub struct CaptureSpans; pub(crate) struct SlotMinting; pub struct SlotPattern; pub struct SlotSubject; pub trait CaptureOutput |
| domain/closure | 4 | pub(crate) function evaluate; pub struct PartitionPlace; pub trait ClosureDriver; pub trait ClosureEvaluation |
| domain/contract | 16 | pub enum ContractError; pub enum FirstOccurrence; pub enum PortableOperatorFault; pub enum PredicateOutcome; pub enum RenderKind; pub enum RightColumns; pub enum RowRead; pub enum TextFound; pub struct GroupSummary; pub struct RangeScratch; pub struct RowScratch; pub struct SourceByteLength; pub struct TextScratch; pub struct VertexPlace; pub(crate) trait Occurrences; pub trait RowDriver |
| domain/cycles | 11 | pub enum CycleMark; pub(crate) function cycles; pub(crate) function evaluate; pub struct CycleScratch; pub struct EdgePlace; pub struct MeetOrder; pub struct NodeCount; pub struct NodeNumber; pub struct WalkStep; pub trait CycleDriver; pub trait CyclesEvaluation |
| domain/equality_join | 15 | pub enum Advance; pub enum EqualityJoinError; pub enum JoinInput; pub enum JoinRow; pub(crate) function evaluate; pub(crate) function join_row; pub(crate) function positions; pub struct CachedReach; pub(crate) struct JoinShape; pub struct PositionSlots; pub struct PositionTarget; pub trait JoinCells; pub trait JoinDriver; pub trait JoinEvaluation; pub trait PositionJoinDriver |
| domain/evaluation | 6 | pub enum EvaluationInput; pub enum Lent; pub(crate) function evaluate; pub struct EvaluationScratch; pub struct EvaluationViews; pub trait InvocationDriver |
| domain/fits_rendered | 9 | pub enum Fit; pub(crate) function evaluate; pub(crate) function fits; pub struct CandidateText; pub(crate) struct LineColumn; pub struct LinePrefix; pub struct LineWidth; pub(crate) struct RenderedShape; pub struct TabStop |
| domain/ordered_dfs | 5 | pub(crate) function evaluate; pub(crate) struct DfsShape; pub struct OrderedEdge; pub trait DfsDriver; pub trait DfsEvaluation |
| domain/ordered_existence | 5 | pub enum PartitionExtremum; pub(crate) function flag; pub struct ExtremumWitness; pub struct OrderedExistenceScratch; pub trait OrderedExistenceDriver |
| domain/ordered_match | 5 | pub enum KeyOrder; pub(crate) function evaluate; pub(crate) function pairs; pub trait OrderedMatchDriver; pub trait OrderedMatchEvaluation |
| domain/project | 2 | pub(crate) function evaluate; pub function project |
| domain/rank | 2 | pub(crate) function evaluate; pub(crate) function rank |
| domain/reduction | 21 | pub enum AggregationCheckOrder; pub enum AggregationPlan; pub enum DigestDatum; pub enum Finishing; pub enum GroupColumns; pub(crate) enum GroupMembership; pub(crate) function check_program; pub(crate) function count_groups; pub(crate) function digest_groups; pub(crate) function finish; pub(crate) function hash_of; pub(crate) function push; pub struct AggregationHash; pub struct GroupCoordinate; pub struct GroupCounts; pub struct GroupDigest; pub struct HashScratch; pub struct ValueHash; pub trait DigestValues; pub trait GroupOutput; pub trait StreamingReduction |
| domain/render_batch | 5 | pub enum RenderAnswer; pub(crate) function deliver; pub(crate) function evaluate; pub(crate) struct RenderMinting; pub trait RenderedOutput |
| domain/replace | 5 | pub(crate) function evaluate; pub(crate) function replace_compiled; pub struct ReplacedText; pub struct ReplacementText; pub(crate) type-alias VisitMatches |
| domain/select | 3 | pub enum RowSelection; pub(crate) function evaluate; pub(crate) function select |
| domain/set_rows | 11 | pub enum RightMembership; pub enum RightPopulation; pub enum SetInput; pub enum SetKind; pub enum SetRowsError; pub(crate) function evaluate; pub(crate) function positions; pub(crate) function values; pub trait SetPositionsDriver; pub trait SetRowDriver; pub trait SetRowsEvaluation |
| domain/source_matches | 17 | pub enum BaseLine; pub enum CaptureRequest; pub(crate) enum CarriageReturn; pub(crate) function evaluate; pub(crate) function occurrences; pub struct CheckedSource; pub struct LineBytes; pub(crate) struct LineCursor; pub(crate) struct OccurrenceByte; pub(crate) struct OccurrenceSpan; pub(crate) struct SearchShape; pub(crate) struct SearchViews; pub struct SourceBase; pub struct SourceMatchesInput; pub trait TextRowDriver; pub trait TextRowsEvaluation; pub(crate) type-alias VisitOccurrences |
| domain/span_between | 1 | pub function between |
| domain/span_bounds | 1 | pub function bounds |
| domain/table_rows | 5 | pub(crate) function evaluate; pub(crate) function rows; pub(crate) struct RowMinting; pub struct RowTexts; pub trait TableRowOutput |
| domain/words | 5 | pub(crate) function evaluate; pub(crate) function split; pub(crate) struct WordMinting; pub struct WordScratch; pub trait WordOutput |
| inbound/use_cases/operators | 4 | pub enum OperatorAnswer; pub enum OperatorCall; pub struct OperatorsUseCaseImpl; pub trait OperatorsUseCase |
| outbound/matching | 2 | pub function use_case; pub trait MatchingPort |
| outbound/matching/adapters/use_case | 1 | pub(crate) struct MatchingUseCaseAdapter |
