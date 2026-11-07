# Route differential

Each execution strategy is unwired alone, and then all of them, and the same workloads run through the same engine.
`results.json` holds one row for each configuration and workload: whether the report is byte for byte the wired engine's
(rows, order and errors), its wall seconds, CPU seconds and peak resident bytes.

The workloads:

- `corpus-413`: the 413 Ruff default rules of the pinned profile over the real-code corpus the python provider tests read
  (`languages/python/tests/fixtures/corpus/*.py`: CPython, FastAPI and requests modules), through the production worker and
  SDK path, with a cold state directory. It is not the 11,185-case inventory: that is run by the shared collector, which
  cannot select a strategy because the selection is a composition argument and not a flag.
- `django-n999`: the runbook's Django canary: rule N999 over the sealed Django corpus of the cold request
  `/tmp/ryo-event-ready-django213-20261105/cold-request.json`, with a fresh cache.

The test is `every_strategy_unwired_answers_like_the_wired_engine` of the `ruff_sharing` target
(`cargo test -p idiomatic-sh-ryo-python --test ruff_sharing every_strategy`); it runs every configuration of both
workloads, so it is run on a quiet machine, never beside another build. The strategy table (parity, 413 wall, Django wall,
RSS, kept or deleted) is in the Phase 5 row of `crates/ryo/docs/contracts/README.md`.
