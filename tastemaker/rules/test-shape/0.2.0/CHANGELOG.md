# test-shape 0.2.0

A new required param `reads` says which test files an entry holds to the runner shape: `every-test` (as 0.1.0 did, for a test beside one source file) or `case-tables`, only a file that declares a `CASES` table. A group test is one of two forms chosen by that one mechanical fact: a case table, held to the shape, or plain `#[test]` scenarios under `scenario-test`. Scenarios `case-tables-only-reads-a-case-table` and `case-tables-only-leaves-a-scenario`; the 0.1.0 scenarios hold with `reads = "every-test"`.
