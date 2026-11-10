# Continue the existing work

Read [AGENTS.md](AGENTS.md). Use the existing Beads checkpoint, not a new task ledger:

```sh
bd prime --export --memories-only
bd show ryo-7tkn.19 --long
bd comments ryo-7tkn.19
```

The [contract matrix](crates/ryo/docs/contracts/README.md) and
[execution plan](crates/ryo/docs/contracts/execution-plan.md) define the work.
Latest user direction: consolidate phases **2–6 first**, then repair output
correctness in phase 1 and finish phase 7. No performance runs during 2–6.
Mandatory ownership and architecture remain enforced throughout.

Verify the checkpoint's actual worktree, Git state and live process handles before
editing or restarting commands. Integrate cohesive agent batches, then compile
and repair together. Preserve incomplete branches; no compatibility bypasses.
