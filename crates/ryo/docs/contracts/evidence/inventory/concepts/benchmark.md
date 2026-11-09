# Concepts: benchmark

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | case_execution | 285 | 3 | service, types |
| application | case_observation | 382 | 1 | service |
| application | case_preparation | 199 | 2 | service |
| application | case_selection | 77 | 3 | service, types |
| application | certification_probe | 54 | 3 | service, types |
| application | certified_measurement | 159 | 3 | service, types |
| application | discrepancy_attachment | 325 | 2 | service |
| application | discrepancy_validation | 404 | 2 | service |
| application | input_metadata | 28 | 2 | service |
| application | inventory_verification | 466 | 2 | service |
| application | measurement | 92 | 3 | service, types |
| application | process_discrepancy | 164 | 2 | service |
| application | publication | 31 | 1 | service |
| application | receipt_validation | 140 | 2 | service |
| application | registered_certification | 168 | 2 | service |
| application | suite_auditing | 81 | 2 | service |
| application | suite_staging | 70 | 1 | service |
| application | worker_identity | 50 | 2 | service |
| application | worker_request | 194 | 3 | service, types |
| domain | candidate_rules | 221 | 3 | service, types |
| domain | canonical_text | 129 | 3 | service, types |
| domain | corpus_discrepancy | 344 | 4 | service, types |
| domain | diagnostics | 282 | 2 | service |
| domain | discrepancy | 397 | 4 | service, types |
| domain | input_metadata | 79 | 2 | service |
| domain | inventory_comparison | 123 | 2 | service |
| domain | inventory_receipts | 516 | 3 | service, types |
| domain | invocation_identity | 84 | 4 | service, types |
| domain | measurements | 193 | 7 | service, types |
| domain | observations | 72 | 1 | service |
| domain | process_failure | 235 | 4 | service, types |
| domain | process_output | 104 | 4 | service, types |
| domain | process_receipts | 401 | 2 | service |
| domain | project_inputs | 105 | 2 | service |
| domain | registered_inputs | 108 | 3 | service, types |
| domain | rule_context | 208 | 3 | service, types |
| domain | suite | 767 | 9 | service, types |
| inbound | use_cases/account_defaults | 60 | 3 | interface, use_case |
| inbound | use_cases/admit_measurement | 123 | 5 | interface, service, types, use_case |
| inbound | use_cases/audit | 72 | 3 | interface, use_case |
| inbound | use_cases/certify | 689 | 6 | interface, service, types, use_case |
| inbound | use_cases/check_worker_request | 98 | 4 | interface, types, use_case |
| inbound | use_cases/collect | 105 | 4 | interface, use_case |
| inbound | use_cases/compare | 117 | 4 | interface, service, use_case |
| inbound | use_cases/compare_corpus | 394 | 6 | interface, service, types, use_case |
| inbound | use_cases/measure | 205 | 4 | interface, service, use_case |
| inbound | use_cases/measure_corpus | 697 | 6 | interface, service, types, use_case |
| inbound | use_cases/prepare_cases | 87 | 3 | interface, use_case |
| inbound | use_cases/publish | 61 | 3 | interface, use_case |
| inbound | use_cases/run_ryo | 127 | 3 | interface, use_case |
| inbound | use_cases/select_cases | 210 | 6 | interface, service, types, use_case |
| inbound | use_cases/stage_suite | 98 | 3 | interface, use_case |
| inbound | use_cases/validate_case_receipts | 99 | 4 | interface, use_case |
| inbound | use_cases/verify_cases | 347 | 4 | interface, service, use_case |
| inbound | use_cases/verify_inventory | 153 | 4 | interface, use_case |
| inbound | use_cases/verify_reference_cases | 251 | 4 | interface, service, use_case |
| outbound | collection | 56 | 5 | interface, types |
| outbound | collection/adapters/json | 228 | 2 | adapter |
| outbound | context | 97 | 5 | interface, types |
| outbound | context/adapters/local | 584 | 4 | adapter |
| outbound | discovery | 32 | 4 | interface |
| outbound | discovery/adapters/listing | 65 | 2 | adapter |
| outbound | engine | 249 | 9 | interface, types |
| outbound | engine/adapters/local | 895 | 4 | adapter |
| outbound | evidence | 102 | 7 | interface, types |
| outbound | evidence/adapters/bmf | 524 | 4 | adapter |
| outbound | execution | 174 | 10 | interface, types |
| outbound | execution/adapters/direct | 120 | 3 | adapter |
| outbound | execution/adapters/process | 150 | 2 | adapter |
| outbound | execution/adapters/timed | 246 | 4 | adapter |
| outbound | inventory | 351 | 5 | interface, types |
| outbound | inventory/adapters/local | 1426 | 13 | adapter, types |
| outbound | linter | 126 | 5 | interface, types |
| outbound | linter/adapters/ryo | 985 | 9 | adapter, types |
| outbound | receipts | 100 | 6 | interface, types |
| outbound | receipts/adapters/local | 1250 | 11 | adapter |
| outbound | suite | 541 | 6 | interface, types |
| outbound | suite/adapters/local | 2829 | 23 | adapter |
| outbound | worker | 55 | 5 | interface, types |
| outbound | worker/adapters/json | 771 | 10 | adapter, types |
| outbound | workspace | 85 | 5 | interface, types |
| outbound | workspace/adapters/git | 393 | 4 | adapter, types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 153 |
