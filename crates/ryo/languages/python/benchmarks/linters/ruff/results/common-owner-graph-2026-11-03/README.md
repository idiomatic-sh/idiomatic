# Common owner graph candidate: matched workload

Frozen engine sources: base `f09b48b3a924e20b36987f74fd6100dbbc84cd66`, graph `fef20a0123ca59ab74a09cfa014501c40ba722cd`. The [Django summary](django-summary.json), [Rust summary](rust-summary.json), [identity/recipe manifest](metadata.json), and per-phase receipts here bind a single pair on the same Python 3.10 target, 310 rules, 2,875 Django files, one Rayon thread. Every paired Django diagnostic report and coverage set matched exactly. An earlier contended cold run is excluded from timing.

| Phase | Base | Graph | Change |
| --- | ---: | ---: | ---: |
| Django cold wall | 240.399 s | 241.331 s | +0.39% |
| Django warm wall | 2.058 s | 2.328 s | +13.08% |
| Django retained edit wall | 9.440 s | 8.381 s | −11.22% |
| Django cold RSS | 2.260 GB | 2.267 GB | +0.32% |
| Rust cold | 1,683 ms | 1,650 ms | −1.96% |
| Rust spike CPU | 2,462 ms | 2,508 ms | +1.87% |

- Core 526/526 and four capability targets 11/11 passed; the [final domain review](domain-review.json) passed. This proves scoped graph/domain correctness controls, not all ownership routes.
- Cold performance is effectively flat; warm regressed in this single pair and retained edit improved. No general 20–50% gain is established. The earlier Rust spike guard and separate warm 100 MB cap remain open.
- Fanout and terminal ownership are still open. The [frozen recipes](driver-recipe.py) and per-run manifests identify the measurements; summary values are observations, not CPU attribution.
