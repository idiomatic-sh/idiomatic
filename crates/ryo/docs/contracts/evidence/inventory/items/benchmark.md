# Items: benchmark

One row per concept: every struct, enum, trait, function and type alias written with a visibility other than private, as `visibility kind name`.

| concept | items | items defined |
|---|---|---|
| application/case_execution | 3 | pub function candidate; pub(crate) function run; pub(crate) struct AttemptOutcome |
| application/case_observation | 1 | pub(crate) function write |
| application/case_preparation | 1 | pub(crate) function cases |
| application/case_selection | 2 | pub function select; pub struct ShardSelection |
| application/certification_probe | 2 | pub function probe; pub struct ProbedProfiles |
| application/certified_measurement | 5 | pub function admit; pub function conclude; pub function pair; pub function require_covered; pub struct RunBaseline |
| application/discrepancy_attachment | 2 | pub(crate) enum DiscrepancyAttachmentError; pub(crate) function attach |
| application/discrepancy_validation | 5 | pub(crate) enum DiscrepancyValidationError; pub(crate) function settings_text; pub(crate) function validate; pub(crate) function validate_all; pub(crate) function validate_archives |
| application/input_metadata | 1 | pub function admit |
| application/inventory_verification | 4 | pub function certify_registered; pub function require_authority; pub function verify; pub function verify_context |
| application/measurement | 3 | pub(crate) enum Pinning; pub function run; pub(crate) function run_with |
| application/process_discrepancy | 3 | pub(crate) enum ProcessDiscrepancyError; pub(crate) function matches; pub(crate) function matches_capture |
| application/publication | 1 | pub function publish |
| application/receipt_validation | 1 | pub function validate |
| application/registered_certification | 2 | pub function admit; pub function pin_rule |
| application/suite_auditing | 1 | pub(crate) function audit |
| application/suite_staging | 1 | pub(crate) function stage |
| application/worker_identity | 1 | pub function check |
| application/worker_request | 3 | pub function configure; pub function prepare; pub struct PreparedWorkerRequest |
| domain/candidate_rules | 6 | pub(crate) function exclude_configuration; pub(crate) function exclude_missing; pub(super) function frontend_exclusions; pub(crate) function project; pub(crate) function retain_producers; pub(crate) struct CandidateConfiguration |
| domain/canonical_text | 7 | pub(crate) enum Fragment; pub(crate) function digest; pub(crate) function render; pub(crate) function text_digest; pub(crate) struct Document; pub(crate) struct Label; pub(crate) struct Quantity |
| domain/corpus_discrepancy | 9 | pub(crate) enum CorpusDiscrepancyError; pub(crate) enum ProblemVerdict; pub(crate) function admit_accepted; pub(crate) function corpus_digest; pub(crate) function corpus_problems; pub(crate) function outside_problems; pub(crate) function skip_problems; pub(crate) struct AdmittedDifferences; pub(crate) struct ObservedDifferences |
| domain/diagnostics | 7 | pub function compare; pub function compare_diagnostics; pub function compare_dimensions; pub(crate) function compare_rules; pub function compare_with_contract; pub(crate) function position; pub function validate_coverage |
| domain/discrepancy | 19 | pub(crate) enum Contestant; pub(crate) enum DiscrepancyError; pub(crate) enum IntentAgreement; pub(crate) enum OptionalDigest; pub(crate) enum ReferenceKind; pub(crate) function check_evidence; pub(crate) function complete; pub(crate) function describe_difference; pub(crate) function identity_digest; pub(crate) function judge; pub(crate) function matches_intended; pub(crate) function output_digest; pub(crate) function reference_digest; pub(crate) function reference_expected; pub(crate) function require_distinct; pub(crate) function validate_pins; pub(crate) struct IdentityText; pub(crate) struct Judgement; pub(crate) struct ReferenceProof |
| domain/input_metadata | 3 | pub(crate) enum InputMetadataError; pub function digest; pub function require |
| domain/inventory_comparison | 2 | pub(crate) enum InventoryComparisonError; pub(crate) function compare |
| domain/inventory_receipts | 6 | pub(crate) enum InventoryReceiptsError; pub(crate) function registered_inputs; pub(crate) function validate; pub(crate) function validate_with_artifacts; pub(crate) struct ReceiptCounts; pub(crate) struct Tally |
| domain/invocation_identity | 6 | pub(crate) enum InvocationIdentityError; pub(crate) function boundaries; pub(crate) function normalize; pub(crate) function normalize_text; pub(crate) struct Boundaries; pub(crate) struct InvocationText |
| domain/measurements | 4 | pub enum MeasurementEligibility; pub function require_inventory; pub struct BenchmarkSample; pub struct Pair |
| domain/observations | 2 | pub(crate) function closure; pub(crate) function project_profile |
| domain/process_failure | 7 | pub(crate) enum CaptureRecord; pub(crate) enum ProcessFailureError; pub(crate) function check_archives; pub(crate) function expected; pub(crate) function group_digest; pub(crate) function output_digest; pub(crate) struct ProcessCapture |
| domain/process_output | 4 | pub enum ProcessOutputError; pub(crate) function expand; pub(crate) function observation_key; pub(crate) struct ReceiptKeyParts |
| domain/process_receipts | 2 | pub(crate) enum ProcessReceiptsError; pub(crate) function validate |
| domain/project_inputs | 1 | pub function digest |
| domain/registered_inputs | 6 | pub(crate) enum ObservationDeclaration; pub(crate) enum RegisteredInputsError; pub(crate) function metadata_digest; pub(crate) function project_inputs_digest; pub(crate) struct RefusalText; pub(crate) struct RegisteredInputs |
| domain/rule_context | 7 | pub function admit_configuration; pub function check; pub function check_selection; pub function literal_glob; pub function resolve; pub function selected_pairs; pub struct LiteralGlob |
| domain/suite | 21 | pub enum CaseOwnership; pub enum ContextAbsence; pub enum InventoryVerdict; pub enum RuleEvidence; pub(crate) function audited; pub(crate) function context_input_path; pub function declares_missing; pub(crate) function expected_report; pub function materialized_sources; pub function owns_cases; pub function require_accounted; pub function require_complete; pub function validate_case_sources; pub function validate_cases; pub function validate_selected_rules; pub struct Audit; pub struct Certificate; pub struct FailureReason; pub struct Inventory; pub struct PassedRule; pub struct RuleResult |
| inbound/use_cases/account_defaults | 2 | pub struct AccountDefaultsUseCaseImpl; pub trait AccountDefaultsUseCase |
| inbound/use_cases/admit_measurement | 4 | pub(super) function admit; pub struct AdmitMeasurementUseCaseImpl; pub struct MeasurementAdmission; pub trait AdmitMeasurementUseCase |
| inbound/use_cases/audit | 2 | pub struct AuditUseCaseImpl; pub trait AuditUseCase |
| inbound/use_cases/certify | 6 | pub enum PreparedCases; pub(super) function certify; pub(super) function prepare; pub struct CertifyUseCaseImpl; pub struct RegisteredContext; pub trait CertifyUseCase |
| inbound/use_cases/check_worker_request | 3 | pub struct CheckWorkerRequestUseCaseImpl; pub struct WorkerExpectation; pub trait CheckWorkerRequestUseCase |
| inbound/use_cases/collect | 2 | pub struct CollectUseCaseImpl; pub trait CollectUseCase |
| inbound/use_cases/compare | 3 | pub(super) function compare; pub struct CompareUseCaseImpl; pub trait CompareUseCase |
| inbound/use_cases/compare_corpus | 6 | pub(super) function compare_corpus; pub struct CompareCorpusUseCaseImpl; pub struct CorpusParity; pub(super) struct ParityBoundaries; pub(super) struct SourceTexts; pub trait CompareCorpusUseCase |
| inbound/use_cases/measure | 4 | pub(super) function measure; pub(super) function measure_certified; pub struct MeasureUseCaseImpl; pub trait MeasureUseCase |
| inbound/use_cases/measure_corpus | 14 | pub enum SourceProfile; pub(super) function measure_corpus; pub(super) struct ComparisonName; pub(super) struct CorpusBoundaries; pub struct CorpusLanguage; pub struct CorpusMeasurement; pub(super) struct FindingCounts; pub struct LanguageTitle; pub struct MeasureCorpusUseCaseImpl; pub struct ProfileLabel; pub struct SampleLabel; pub struct SourceSuffix; pub struct ToolTitle; pub trait MeasureCorpusUseCase |
| inbound/use_cases/prepare_cases | 2 | pub struct PrepareCasesUseCaseImpl; pub trait PrepareCasesUseCase |
| inbound/use_cases/publish | 2 | pub struct PublishUseCaseImpl; pub trait PublishUseCase |
| inbound/use_cases/run_ryo | 2 | pub struct RunRyoUseCaseImpl; pub trait RunRyoUseCase |
| inbound/use_cases/select_cases | 6 | pub enum SelectedIds; pub(super) function select; pub struct CaseArgument; pub struct CaseRun; pub struct SelectCasesUseCaseImpl; pub trait SelectCasesUseCase |
| inbound/use_cases/stage_suite | 2 | pub struct StageSuiteUseCaseImpl; pub trait StageSuiteUseCase |
| inbound/use_cases/validate_case_receipts | 2 | pub struct ValidateCaseReceiptsUseCaseImpl; pub trait ValidateCaseReceiptsUseCase |
| inbound/use_cases/verify_cases | 3 | pub(super) function verify_prepared_shard; pub struct VerifyCasesUseCaseImpl; pub trait VerifyCasesUseCase |
| inbound/use_cases/verify_inventory | 2 | pub struct VerifyInventoryUseCaseImpl; pub trait VerifyInventoryUseCase |
| inbound/use_cases/verify_reference_cases | 3 | pub(super) function verify_prepared_reference_cases; pub struct VerifyReferenceCasesUseCaseImpl; pub trait VerifyReferenceCasesUseCase |
| outbound/collection | 3 | pub function json; pub struct CollectorMessage; pub trait CollectionPort |
| outbound/collection/adapters/json | 5 | pub(super) function invocation; pub(super) function observation; pub(super) function request; pub(super) function response; pub(crate) struct CollectionJsonAdapter |
| outbound/context | 3 | pub function local; pub struct CorpusFiles; pub trait ContextPort |
| outbound/context/adapters/local | 15 | pub(super) enum PathScope; pub(crate) function artifact; pub(crate) function fingerprint; pub(super) function frame; pub(crate) function materialize; pub(super) function no_symlinks; pub(crate) function receipt; pub(super) function receipt_text; pub(super) function relative; pub(super) function resolve; pub(crate) function seal_existing; pub(crate) function validate; pub(crate) function verify_receipt; pub(super) function walked_dependency; pub(crate) struct ContextLocalAdapter |
| outbound/discovery | 2 | pub function listing; pub trait DiscoveryPort |
| outbound/discovery/adapters/listing | 1 | pub(crate) struct DiscoveryListingAdapter |
| outbound/engine | 12 | pub function local; pub function local_with_diagnostic_events; pub function local_with_frontend_diagnostic; pub struct ConfiguredEngineSession; pub struct EngineConfiguration; pub struct EngineRun; pub struct FrontendDiagnosticProjection; pub struct LoadedEngineSession; pub struct PreparedMissingInputs; pub struct SelectedPairs; pub(crate) trait EngineExecution; pub trait EnginePort |
| outbound/engine/adapters/local | 9 | pub(super) function load_rules; pub(super) function open; pub(super) function refuse_incomplete; pub(super) function source_inputs; pub(super) function subscriber; pub(super) function validate_skips; pub(super) struct DiagnosticEventWriter; pub(crate) struct EngineLocalAdapter; pub(super) struct LockedDiagnosticEventWriter |
| outbound/evidence | 3 | pub function bmf; pub struct MeasuredPublication; pub trait EvidencePort |
| outbound/evidence/adapters/bmf | 6 | pub(super) function comparison; pub(super) function encode; pub(super) function parity_metrics; pub(super) function parity_rules; pub(crate) struct EvidenceBmfAdapter; pub(crate) struct EvidenceBmfConnection |
| outbound/execution | 9 | pub enum ExecutionError; pub enum ExitObservationMode; pub function bounded; pub(crate) function decode; pub function direct; pub function process; pub function timed; pub(crate) struct ProcessExit; pub trait ExecutionPort |
| outbound/execution/adapters/direct | 2 | pub(super) function decode; pub(crate) struct ExecutionDirectAdapter |
| outbound/execution/adapters/process | 1 | pub(crate) struct ExecutionProcessAdapter |
| outbound/execution/adapters/timed | 4 | pub(super) function observation; pub(super) function request; pub(crate) struct ExecutionTimedAdapter; pub(crate) struct ExecutionTimedSettings |
| outbound/inventory | 25 | pub enum Command; pub enum ReceiptDigest; pub enum ReceiptFile; pub enum RegistrationAuthority; pub enum RowInstance; pub enum RowReason; pub enum RowStatus; pub enum RuleCommand; pub enum RuleDefinition; pub enum RuleInstance; pub enum RuleReason; pub enum RunState; pub function local; pub struct ContextFiles; pub struct ExecutablePin; pub struct FilePin; pub struct IdentityKey; pub struct ProviderName; pub struct RawArtifacts; pub struct RegisteredModuleName; pub struct RegisteredRule; pub struct Registration; pub struct Row; pub struct Summary; pub trait InventoryPort |
| outbound/inventory/adapters/local | 8 | pub(crate) struct AuditJson; pub(crate) struct CaseText; pub(crate) struct DiscrepancyText; pub(crate) struct InventoryLocalAdapter; pub(crate) struct InvocationBytes; pub(crate) struct RegistrationBytes; pub(crate) struct RowJson; pub(crate) struct SummaryJson |
| outbound/linter | 4 | pub function ryo; pub function ryo_context; pub struct CanonicalText; pub trait LinterPort |
| outbound/linter/adapters/ryo | 5 | pub(in crate::outbound::linter::adapters::ryo) function observation_declarations_value; pub(in crate::outbound::linter::adapters::ryo) function prepared_value; pub(crate) struct LinterRyoAdapter; pub(crate) struct ReportBytes; pub(crate) struct RequestJson |
| outbound/receipts | 6 | pub enum CaseReceipts; pub enum ReceiptsError; pub function local; pub struct CapturedProcess; pub struct ReceiptPath; pub trait ReceiptsPort |
| outbound/receipts/adapters/local | 9 | pub(in super::super) enum KindSchema; pub(in super::super) function bytes; pub(in super::super) function pin; pub(super) function read; pub(super) function record; pub(in super::super) function row_bytes; pub(in super::super) function rows; pub(in super::super) struct ProcessRowSchema; pub(crate) struct ReceiptsLocalAdapter |
| outbound/suite | 26 | pub enum ArtifactMetadata; pub enum ClaimedEvidence; pub enum ClaimedMetadata; pub enum ClaimedProcess; pub enum EvidenceRequirement; pub enum MaterializedContext; pub enum SuiteError; pub function local; pub struct ArtifactBytes; pub struct ArtifactPath; pub struct ArtifactSet; pub struct ArtifactSize; pub struct ArtifactText; pub struct CaseArtifacts; pub struct CaseClaim; pub struct ClaimedDiscrepancy; pub struct ClaimedFinding; pub struct ClaimedIdentity; pub struct EvidenceArtifact; pub struct MaterializedCase; pub struct RetainedDirectory; pub struct RuleQuery; pub struct SelectedName; pub struct SourceDigest; pub trait ScopedDirectory; pub trait SuitePort |
| outbound/suite/adapters/local | 38 | pub(in super::super) enum OutputPartSchema; pub(in super::super) enum PathEncodingSchema; pub(in super::super) enum ProcessFailureSchema; pub(in super::super) function accepted; pub(in super::super) function actual_digest; pub(in super::super) function case_claims; pub(super) function case_claims; pub(in super::super) function cases; pub(super) function copy_evidence; pub(super) function fingerprint_path; pub(super) function materialize; pub(super) function materialize_inventory; pub(in super::super) function names; pub(in super::super) function narrowed; pub(in crate::outbound::suite::adapters::local) function observation_declarations; pub(in super::super) function observed; pub(in super::super) function observed; pub(in crate::outbound::suite::adapters::local) function project_search_roots; pub(super) function relative; pub(in super::super) function rule; pub(super) function rule_directory; pub(super) function stage; pub(in super::super) function stage_executable; pub(super) function stage_projects; pub(in super::super) function suite; pub(in super::super) function tool; pub(in super::super) function validate; pub(super) function validate_query_modules; pub(in super::super) function verify_requirement; pub(in crate::outbound::suite::adapters::local) struct CaseContextSchema; pub(in crate::outbound::suite::adapters::local) struct ObservationBindingSchema; pub(in crate::outbound::suite::adapters::local) struct OptionsSchema; pub(in super::super) struct ProcessObservationSchema; pub(in crate::outbound::suite::adapters::local) struct ProjectInventorySchema; pub(in super::super) struct ReferenceDiscrepancySchema; pub(in crate::outbound::suite::adapters::local) struct RuleConfigurationSchema; pub(in crate::outbound::suite::adapters::local) struct RuleContextDeclarationSchema; pub(crate) struct SuiteLocalAdapter |
| outbound/worker | 3 | pub enum WorkerSource; pub function json; pub trait WorkerPort |
| outbound/worker/adapters/json | 5 | pub(crate) struct DocumentWire; pub(crate) struct WorkerJsonAdapter; pub(crate) struct WorkerJsonConnection; pub(crate) struct WorkerPayload; pub(crate) struct WorkerResponseBytes |
| outbound/workspace | 6 | pub function git; pub struct AppliedPatch; pub struct CommitRevision; pub struct TrackedFiles; pub struct Workspace; pub trait WorkspacePort |
| outbound/workspace/adapters/git | 5 | pub(super) struct GitArguments; pub(super) struct GitInput; pub(super) struct GitOutput; pub(crate) struct WorkspaceGitAdapter; pub(crate) struct WorkspaceGitSettings |
