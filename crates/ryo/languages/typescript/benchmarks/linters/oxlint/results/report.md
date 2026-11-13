# Oxlint benchmark results

## Fused engine, 18 rules (2026-11-13)

Source `claude/checkpoint-341` (spike `50fd0800ac` with `claude/measure-subset` `a1a5e5e619` and `claude/oxlint-bench` `72087c14a3` merged), ARM64 VM with 10 cores. Each build of the worker ran `diagnostic-measure-valid <oxlint 1.76.0> <fresh cache> <collector>` once, with a 1-minute load under 3 at the start (main 0.94, fused 1.02). The two builds differ only in `--features idiomatic-sh-ryo-core/fused-evaluation`. Certification passed 178 cases on main. The route runs six trials per project with Ryo pinned to one worker. The rows are medians of the six trials. This is an 18-rule comparison.

| project | condition | oxlint wall / CPU-s / RSS | Ryo main wall / CPU-s / RSS | Ryo fused |
|---|---|---:|---:|---|
| hono, 352 files | empty tool cache | 0.024 s / 0.023 / 22 MB | 4.151 s / 4.150 / 54 MB | refused |
| hono | retained tool cache | 0.023 s / 0.023 / 22 MB | 0.016 s / 0.016 / 20 MB | refused |
| hono | edited tool cache | 0.023 s / 0.023 / 22 MB | 0.066 s / 0.066 / 26 MB | refused |
| vite, 617 files | empty tool cache | 0.038 s / 0.037 / 24 MB | 7.962 s / 7.960 / 56 MB | refused |
| vite | retained tool cache | 0.036 s / 0.036 / 24 MB | 0.018 s / 0.018 / 21 MB | refused |
| vite | edited tool cache | 0.037 s / 0.036 / 24 MB | 0.323 s / 0.323 / 39 MB | refused |

Main's findings matched oxlint exactly in every trial: hono none cold, `eslint.no-debugger=1` after the edit; vite `eslint.no-useless-rename=1`. The valid profile excludes vite's deliberately invalid `packages/vite/src/node/ssr/__tests__/fixtures/errors/syntax-error.ts`.

The fused build refuses all 18 rules at certification, each alone and together, because the fused evaluator does not evaluate a `span-between` step. Every rule imports `fragments/eslint-directives.ryoql`, which uses `span-between`. So there are no fused numbers and no fused hono profile to compare.
