# Scoped file DAG execution evidence

The file DAG executor is opt-in (`RYO_FILE_DAG_EXECUTION=1`); the existing executor remains the default. A single sealed worker (SHA-256 `10c69096c33482ef165ec381d8d0830b1e7388faac8b52c4f2777820dc092c17`) ran both variants serially on the same frozen Django 3.10 corpus: 2,875 files and 310 rules, with an independent fresh cache for each variant. Both runs used ordinary `run-ryo`, one Rayon thread, and a resource preflight.

| Variant | Wall | CPU | Peak RSS | Report |
| --- | ---: | ---: | ---: | --- |
| DAG off | 255.309 s | 255.205 s | 2,283,868,160 B | `02b96f5f...` |
| DAG on | 211.496 s | 211.369 s | 2,201,784,320 B | `02b96f5f...` |

The graph-on observation saved **43.812 s wall (17.16%)**, **43.836 s CPU (17.18%)**, and **82,083,840 B peak RSS (3.59%)** in this pair. The complete normalized reports are byte equal: 2,327 diagnostics; 891,250 selected, 890,943 evaluated, 307 explicit syntax skips, and no unknown or unread pairs. Both have zero diagnostic delta from the frozen historical report. These are scoped observations, not a general speed or acceptance claim; the graph remains disabled by default.

An intrusive diagnostic run on `django/contrib/admin/options.py` recorded 8,377 DAG demands, 2,597 shared hits, 5,780 executions, 3,759 releases, and a 6,291,456 B peak retained DAG allocation. The one-file ordinary and diagnostic outputs were byte equal. Its timings are not used as a performance result.

[Pair summary](summary.json) · [Resource preflight](resource-preflight.json) · [Evidence index](evidence-index.json) · [Pair inputs and receipts](raw.tar.gz) · [One-file diagnostic inputs and receipts](one-file-raw.tar.gz).
