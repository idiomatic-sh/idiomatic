# Concepts: benchmark

One row per concept directory (a directory holding service.rs, types.rs, interface.rs, adapter.rs or use_case.rs). Lines are production lines of every non-test file in the directory and its subdirectories that no deeper concept owns.

| layer | concept | production lines | files | marker files |
|---|---|---|---|---|
| application | case_preparation | 145 | 2 | service |
| application | discrepancy_attachment | 306 | 2 | service |
| application | discrepancy_validation | 380 | 3 | service |
| application | inventory_receipts | 481 | 3 | service, types |
| application | process_discrepancy | 138 | 2 | service |
| application | process_receipts | 387 | 3 | service |
| application | run | 1763 | 10 | service |
| application | suite_auditing | 79 | 2 | service |
| application | suite_staging | 61 | 2 | service |
| application | worker_request | 174 | 3 | service, types |
| domain | candidate_rules | 208 | 3 | service, types |
| domain | canonical_text | 127 | 3 | service, types |
| domain | diagnostics | 114 | 2 | service |
| domain | discrepancy | 326 | 4 | service, types |
| domain | invocation_identity | 108 | 4 | service, types |
| domain | measurements | 189 | 7 | service, types |
| domain | process_failure | 216 | 3 | service |
| domain | registered_inputs | 106 | 3 | service, types |
| domain | suite | 999 | 13 | service, types |
| domain | suite/process | 64 | 1 | service |
| inbound | use_cases/audit | 72 | 3 | interface, use_case |
| inbound | use_cases/certify | 600 | 6 | interface, service, use_case |
| inbound | use_cases/collect | 102 | 3 | interface, use_case |
| inbound | use_cases/compare | 136 | 4 | interface, service, use_case |
| inbound | use_cases/measure | 264 | 5 | interface, service, use_case |
| inbound | use_cases/prepare_cases | 51 | 3 | interface, use_case |
| inbound | use_cases/publish | 59 | 3 | interface, use_case |
| inbound | use_cases/run_ryo | 99 | 3 | interface, use_case |
| inbound | use_cases/stage_suite | 75 | 3 | interface, use_case |
| inbound | use_cases/validate_case_receipts | 83 | 3 | interface, use_case |
| inbound | use_cases/verify_cases | 102 | 4 | interface, use_case |
| inbound | use_cases/verify_inventory | 110 | 3 | interface, use_case |
| inbound | use_cases/verify_reference_cases | 64 | 3 | interface, use_case |
| outbound | collection | 56 | 5 | interface, types |
| outbound | collection/adapters/json | 222 | 2 | adapter |
| outbound | context/adapters/local | 703 | 3 | adapter |
| outbound | engine | 234 | 9 | interface, types |
| outbound | engine/adapters/local | 848 | 3 | adapter |
| outbound | evidence | 81 | 8 | interface, types |
| outbound | evidence/adapters/bmf | 185 | 4 | adapter |
| outbound | evidence/adapters/comparison | 65 | 2 | adapter |
| outbound | execution | 125 | 8 | interface |
| outbound | execution/adapters/direct | 118 | 3 | adapter |
| outbound | execution/adapters/process | 144 | 2 | adapter |
| outbound | execution/adapters/timed | 239 | 4 | adapter |
| outbound | inventory | 556 | 7 | interface, types |
| outbound | inventory/adapters/local | 1025 | 4 | adapter |
| outbound | linter | 90 | 4 | interface |
| outbound | linter/adapters/ryo | 387 | 3 | adapter |
| outbound | suite | 1096 | 12 | interface, types |
| outbound | suite/adapters/local | 3097 | 17 | adapter |
| outbound | worker | 269 | 5 | interface |
| outbound | worker/adapters/json | 468 | 5 | adapter, types |
| outbound | workspace | 85 | 5 | interface, types |
| outbound | workspace/adapters/git | 393 | 4 | adapter, types |

Production lines in directories with no concept ancestor:

| layer | production lines |
|---|---|
| inbound | 98 |
| outbound | 10 |
