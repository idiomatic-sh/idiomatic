# Plan: trunk-dissolved-crates.md (mapping job, bead ryo-fmx3.1.13.23.3)

Status: research done for all five crates; the output file is NOT written (plan mode started before the write).
Only write after approval: /workspaces/ryo-poc-evidence/next/build/ports/stage2/mapping/trunk-dissolved-crates.md

## Working material (scratchpad, read-only snapshot of T=2a207464c)
/tmp/claude-1000/-workspaces-idiomatic/3d52d4bd-d543-48d6-87bb-d5a9b9b3e779/scratchpad/ : snap/ (git archive of crates/ at T), users.py (module users), lines.py (lines without tests), tr.py (trait impls/users), sym.py.

## Line totals without tests (lines.py filter)
core 2,969 (brief says 3,127; filter differs) | execution 176 | execution_domain 1,604 | matching 1,090 (brief 1,172) | values 8,549.

## Findings to put in the file

### core
- 6 use cases under inbound/use_cases: describe_operator, describe_relation, list_operators, list_relations, validate, lint.
  Five have an Impl (use_case.rs) and are called by cli and sdk composition files. `LintUseCase` (lint/interface.rs:7, 1 method) has NO implementor and NO caller: the lint dir has no use_case.rs. Verified by grep over all of crates/.
- Second family confirmed: core lint/types.rs defines SourceProvider, LintRequest, Rule, RuleParameter, ParameterValue, ParameterTable, LintReport, ProjectDependency(+State), RuleWork, StepFed, FeatureFed, ProviderFault, Notice (259 lines) and lint/errors.rs LintError (215). The SDK defines its own Rule/LintRequest/Finding/LintReport/LintError (sdk/src/resources/linter/lint/types.rs:103,169,222,281; errors.rs:11). executor has its own ProviderFault (check_session/refusal.rs:77).
  Core's family is used only by: cli/src/outcome/remedy.rs (LintError), cli/src/resources/query/run/command.rs (DEFAULT_LIMIT), 7 cli replay test files (lint::types::Rule/RuleParameter/ParameterValue). Not used by sdk.
- lint/coordinates.rs (46) FindingCoordinates + domain/finish (98; only fill_line_starts, place_in, FileLine/ColumnNumber/LineStarts/Point used) are called by executor (languages.rs:74, check_session.rs, finish/answer.rs) and sdk (languages/contract.rs:7). domain/finish/service.rs also holds many unused imports.
- application/metadata: kernel_metadata() returns MetadataUseCaseImpl; called by describe_operator and list_operators use cases.
- domain/relation_lookup: resolve(known, name); called by describe_relation only.
- outbound ports (all have 1 trait): LanguagePort (9 methods; impls LanguageUseCaseAdapter, LanguageFake; callers: 3 use cases via Arc<dyn>, cli build via factory); CorpusPort (4 methods; impl CorpusIgnoreAdapter; caller sdk walk.rs:44 inventory() and core bench); SchedulerPort (2 methods; impls Rayon, fakes; caller sdk capacity/types.rs, entry.rs on_owner, core bench; benchmark has its OWN SchedulerPort trait); RowSinkPort (3 methods; impls cli RowsSchema, collect adapter, fakes; caller cli query/run/rendering.rs:33; row-shaped: row(), LentCell, KeptRow); MemoryPort (2 methods; impl sysinfo; caller sdk lint/rows.rs:35,43).
- Dead-looking: corpus/adapters/memory.rs (7 lines, not declared in corpus/adapters.rs, its adapter/constants files absent); scheduler sequential adapter (1 doc line); row_sink collect factory has no caller outside its own tests.
- Contract names rows: RowSinkPort (row/KeptRow), LanguagePort::project takes `&mut dyn RowWriter` (ryo_relations).

### execution (176)
- MetadataUseCase (2 methods: operators(), describe(op)); impl MetadataUseCaseImpl; callers: core describe_operator/list_operators + application/metadata. Re-exports Message, Unknown, FeatureRows for core lint types.
- outbound/matching: MatchingPort (2 methods: compile(globs)->GlobsMatcher, parse(glob)); impl globset adapter over ryo_matching; callers: sdk lint/schemas.rs:45 (parse only) and composition.rs:78. `compile` and `GlobsMatcher.matches` (values Matching trait) have NO non-test caller; sdk call_cache.rs compiles path globs with globset directly.

### execution_domain (1,604)
- describing (391) + operator/constants (379): per-operator description texts, `describe(op)`: -> domain/operators (only caller: execution MetadataUseCaseImpl).
- coverage (263), trust (181): partial-read coverage and absence trust; callers: executor check_session* (coverage 4 files, trust 3) and execution(1) -> domain/execute (medium).
- rule_intent (171), rules (27), spelling (107), features (24), operator/types (12): NO caller outside the crate (FeatureRows only re-exported by execution and used by core lint/types.rs). spelling::check_name_spellings: no caller anywhere. rule_intent: refusals of declared rules/observation bindings: candidates execute or lint; "not checked" for final home.

### matching (1,090)
- Split of MatchingUseCase (11 methods; impl MatchingUseCaseImpl):
  - regex over text: compile_regex, matches_regex, visit_regex_matches, visit_regex_captures (domain/matching/service.rs, regex-automata). Callers: query_language kind_check/service.rs:117,128 only (compile_regex, matches_regex). visit_* have no non-test caller. -> operators.
  - text glob (match glob against text value): compile_text_globs, matches_text_globs, matches_text_glob, all of domain/globbing (550: interpreter + regex fast path). Callers: executor finish/plan_select.rs:148, pair_summary/regexes.rs:28,83, finish/symbol_predicate.rs:163, pair_checks.rs:498, finish.rs:4236, query_language kind_check:122. -> comparison operator (Test::GlobAny / NotGlobAny).
  - path glob (globset): compile_globs, parse_glob, match_globs. Callers: execution globset adapter only (:38,:52,:44). -> outbound/corpus.
- Other ryo_matching users: execution 5 files, executor 5, query_language 1, sdk 1 (MatchingError).

### values (8,549) users (files naming the module; t = test files), dependency edges inside values from users.py
Per module users: cycles executor 1; descriptions 85 (cli 5, core 20, execution 2, exec_domain 2, executor 3, query_language 40, sdk 4); files 33 (cli 5, core 19, execution 3, executor 4, sdk 2); finish_plans 53 (executor 17, query_language 26, sdk 1); functions executor 1; hosted 13; lineage query_language 1; observations 19; plan_orders query_language 4; plan_parts 88; plans 31 (query_language only); program 47; query_bindings 9; query_operators 20; query_plans 0 external; query_requests 7; query_syntax 47 (query_language only); query_tokens 84; rank executor 1; records 10; row_texts 18; rows 3; rule_requests 3 (core 2); run_metadata 2 (core 1, exec_domain 1); runs 31; stored_facts 14; stored_findings 0 external; stores 0 external; table_words 2 (benchmark 1); words executor 1; written_rows query_language 1. profile: executor 1 (re-exported as operators).
Proposed destinations (to be tabled): query_syntax, query_tokens, program, query_requests(?) -> ryoql; plans, plan_parts, finish_plans, plan_orders, query_plans, written_rows, lineage -> plan; query_operators -> operators; descriptions -> split (names to relations/ryoql, Operator/Relation/Language/Grammar/ValidateReport to inbound/describe); records, stored_facts, stored_findings, stores, table_words, row_texts(part), words -> store/cache/execute by concept (low-medium); files -> split corpus / language contract (Matching trait + Glob types -> corpus); runs, rule_requests, run_metadata, observations, hosted, query_bindings -> lint / execute (medium-low); profile -> tools/profile_tools or execute (low); cases.rs -> NO HOME (test support used by 5 crates).
Traits: values files::types::Matching (1 method find; blanket impl for closures; no non-test caller), words::types::WordOutput (1 method; impls executor finish/words.rs Words, test Collected; caller values words service).

## Remaining steps after approval
1. Re-run lines.py per directory to fill the `lines` column; write all rows (split core outbound/language/corpus/... by file, finish_plans by sub-part, values big files).
2. Section 2 totals per destination; 3 NO HOME; 4 No caller found (with grep method); 5 Contracts (traits above: 6 core use cases, 5 core ports, MetadataUseCase, MatchingPort, MatchingUseCase, Matching, WordOutput); 6 Import-direction conflicts (domain->outbound: core finish uses ryo_language; values depends on ryo_language and ryo_relations; execution uses values; inbound lint types importing execution use case interface; operators needing plan OperationKind).
3. Final message: at most 12 lines.
