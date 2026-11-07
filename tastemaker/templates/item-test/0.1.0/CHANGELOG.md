# item-test

## 0.1.0

The item test in the one test shape; the collection's shared cases are asked for by `item-contract`.

The template renders the table and the subject and no runners: a runner for a case word is written with the first row of that word (`test-runners-match-rows`).

The template has no `todo!`: its subject answers a failure until the author states it, and it holds the `succeeds` runner, so a scaffolded item test compiles and fails by name until it has rows.
