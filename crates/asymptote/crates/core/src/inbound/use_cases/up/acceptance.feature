Feature: up

  Rule: up records the last check's new violations the selectors choose, and the rest still fail

    @succeeds
    Scenario Outline: up succeeds
      Given the up input "<input>"
      When up runs
      Then up answers "<expected>"

      Examples:
        | name       | input      | expected              |
        | everything | everything | 2 recorded, 0 left    |
        | ignored    | ignored    | 2 recorded, 0 left    |
        | at         | at         | 1 recorded, 1 left    |

    @refuses
    Scenario Outline: up refuses
      Given the up input "<input>"
      When up runs
      Then up answers "<expected>"

      Examples:
        | name      | input     | expected                               |
        | nowhere   | nothing   | no new finding is at `src/a.rs:9`      |
        | unchecked | unchecked | no check has run here with this asymptote |

  Rule: up seeds a perf tool's measures, and raises a regressed best, in one operation with the reason

    @succeeds
    Scenario Outline: up seeds perf records
      When up seeds the measured product "<input>"
      Then up answers "<expected>"

      Examples:
        | input        | expected |
        | seed         | 1 perf recorded; then Pass; best 100; operations 1 |
        | seed-tool    | 1 perf recorded; then Pass; best 100; operations 1 |
        | raise        | 1 perf recorded; then Pass; best 120; operations 2 |
        | many         | 13 perf recorded; then Pass; best -; operations 1 |
        | other-tool   | 0 perf recorded; then Changed; best -; operations 0 |
