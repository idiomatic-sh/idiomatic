Automatic sparse traversal guards
=================================

run.py receives the existing sealed benchmark worker from candidate profile
verification, after its protocol audit. It never builds or invokes Ruff. It runs
the PIE794, B023/C419 and PLR1708 generic membership probes once per candidate
verification invocation, preserving unique raw/report artifacts. Nonzero exits,
missing proof, truncated case coverage, Unknown results and counted-bag
mismatches fail the invocation. Its four unit tests exercise those refusals with
an injected subprocess runner and require no real worker.

The companion verifier insertion is handed integration as
/tmp/sparse-traversal-verifier-hook.patch; the shared helper has exclusive
integration ownership. Registration is complete only once that insertion is
merged. Existing asymptote candidate verification calls verify_profiles.py,
so the hook runs these guards with the worker that job already built. Resume
creates a fresh proof directory instead of trusting stale guard success. The
separate existing core Axis `succeeds` test includes a rooted visited-set work
guard independent of unrelated graph components; that test is pending the next
combined Rust gate, not claimed executed by this Python runner.

executions.json records the initial successful manual registered-runner check on
worker2ea. These generic probes use actual language NodeIds, repeated roots with
different payloads, multiple files and unrelated width/depth growth. They assert
membership and bags, not equivalence of BFS and sorted-closure physical order.
