Feature: check

  Rule: independent cache connections preserve complete results and rebuild invalid cache entries

    @succeeds
    Scenario Outline: a persisted scan is reused only while its inputs and the store remain valid
      When the production cache is reopened after "<change>"
      Then every complete check report matches its expected lifecycle outcome

      Examples:
        | change |
        | no change |
        | source and tool output edit |
        | corrupted store |

  Rule: actual subprocess SARIF controls the product verdict

    @succeeds
    Scenario Outline: successful tool reports keep their SARIF meaning
      Given the check input "<input>"
      When the real SARIF tool is checked
      Then check answers "<expected>"

      Examples:
        | input | expected |
        | indexed | Changed, 1 new |
        | default-indices | Changed, 1 new |
        | pass | Pass, 0 new |

    @fails
    Scenario: unsuccessful SARIF execution cannot become a clean product
      Given the check input "unsuccessful"
      When the real SARIF tool is checked
      Then check answers "Undetermined, 0 new"

  Rule: check judges what the tools report against the lock, and reuses a run whose inputs did not move

    @succeeds
    Scenario Outline: check succeeds
      Given the check input "<input>"
      When check runs
      Then check answers "<expected>"

      Examples:
        | name  | input | expected            |
        | first | first | 2 new, ran          |
        | warm  | warm  | 2 new, reused       |
        | since | since | 2 introduced, ran   |
        | fix fails, check still runs | fix-fails | 2 new, ran; lint fix exited 2: cannot fix |
        | fix times out | fix-times-out | 2 new, ran; lint fix timed out after 60 s |
        | fix succeeds, nothing said | fix-succeeds | 2 new, ran |

    @refuses
    Scenario Outline: check refuses
      Given the check input "<input>"
      When check runs
      Then check answers "<expected>"

      Examples:
        | name    | input   | expected                          |
        | nowhere | nowhere | no asymptote.toml at or above `/elsewhere` |

  Rule: a tool whose inputs capture `{name}` is one tool per match, each reused while its own inputs hold

    @succeeds
    Scenario Outline: a captured tool runs once for each directory, and reruns only where a directory moved
      When the captured product is checked after "<change>"
      Then check answers "<expected>"

      Examples:
        | change            | expected                                        |
        | nothing           | tools unit/a+unit/b; reused unit/a+unit/b        |
        | an edit in a      | tools unit/a+unit/b; reused unit/b               |
        | a new directory c | tools unit/a+unit/b+unit/c; reused unit/a+unit/b |

    @refuses
    Scenario: a capture in run that no input writes is refused
      When the captured product is checked after "a capture no input names"
      Then check answers "`./asymptote.toml`: tool `unit`: `run` uses `{area}`, which no input captures: write it in one of its `inputs`, as in `<directory>/{area}`"

  Rule: a perf tool is judged against the lock's best for each measure

    @succeeds
    Scenario Outline: perf judging succeeds
      When the measured product "<input>" is checked
      Then check answers "<expected>"

      Examples:
        | input       | expected |
        | held        | Pass; ran lint+bench+unit; regressed 0 unseeded 0 improved 0 resolved 0; best 100; operations 1 |
        | improved    | Pass; ran lint+bench+unit; regressed 0 unseeded 0 improved 1 resolved 0; best 70; operations 2 |
        | improved-fixing | Pass; ran lint+bench+unit; regressed 0 unseeded 0 improved 1 resolved 0; best 100; operations 1 |

    @refuses
    Scenario Outline: perf judging refuses
      When the measured product "<input>" is checked
      Then check answers "<expected>"

      Examples:
        | input     | expected |
        | regressed | Changed; ran lint+bench+unit; regressed 1 unseeded 0 improved 0 resolved 0; best 100; operations 1 |
        | unseeded  | Changed; ran lint+bench+unit; regressed 0 unseeded 1 improved 0 resolved 0; best -; operations 0 |
        | resolved  | Changed; ran lint+bench+unit; regressed 0 unseeded 0 improved 0 resolved 1; best 100; operations 1 |

  Rule: check --groups runs only the tools of those groups and judges no record of the others

    @succeeds
    Scenario Outline: groups select the tools that run
      When the measured product "<input>" is checked
      Then check answers "<expected>"

      Examples:
        | input      | expected |
        | lint-group | Pass; ran lint; regressed 0 unseeded 0 improved 0 resolved 0; best 100; operations 1 |
        | perf-group | Changed; ran bench; regressed 1 unseeded 0 improved 0 resolved 0; best 100; operations 1 |
        | two-groups | Changed; ran lint+bench; regressed 0 unseeded 1 improved 0 resolved 0; best -; operations 0 |

    @succeeds
    Scenario Outline: a check of some groups keeps the runs of the others for the next check
      When the measured product "<input>" is checked
      Then check answers "<expected>"

      Examples:
        | input         | expected |
        | all-then-lint | Changed; ran lint+bench+unit; regressed 0 unseeded 1 improved 0 resolved 0; best -; operations 0; reused lint+bench+unit |

    @fails
    Scenario Outline: a failing test tool fails the run and writes nothing to the lock
      When the measured product "<input>" is checked
      Then check answers "<expected>"

      Examples:
        | input        | expected |
        | failing-test | Undetermined; ran lint+bench+unit; regressed 0 unseeded 0 improved 0 resolved 0; best 100; operations 1 |
        | test-group   | Undetermined; ran unit; regressed 0 unseeded 0 improved 0 resolved 0; best -; operations 0 |
