# Full-inventory CI contract review

The [workflow](https://github.com/idiomatic-sh/idiomatic/blob/f3f27d77/.github/workflows/check.yml) runs the pinned Ruff worker, benchmark collector, and all 413 defaults in one independent job. The [contract tests](https://github.com/idiomatic-sh/idiomatic/blob/f3f27d77/ci/workflows_test.py) reject skipped builds, narrower inventories, masked exits, changed checkout or setup actions, and missing receipts.

- [Review 1](review-1.json) blocked missing step and job condition guards; fixed in [426293d4](https://github.com/idiomatic-sh/idiomatic/commit/426293d4).
- [Review 2](review-2.json) blocked unchecked checkout/setup actions and job container; fixed in [f3f27d77](https://github.com/idiomatic-sh/idiomatic/commit/f3f27d77).
- [Review 3](review-3.json) passed on clean `f3f27d77a99c9ed52df2d7f61395a5dbb982b302`.
- Focused check: `uv run --with pytest --with pyyaml python -m pytest ci/workflows_test.py -q` → **68 passed in 0.59s**. No full CI rerun is claimed here.

Approval covers this dedicated job's regression contract. Historical platform and quality failures remain recorded on `ryo-7tkn`.
