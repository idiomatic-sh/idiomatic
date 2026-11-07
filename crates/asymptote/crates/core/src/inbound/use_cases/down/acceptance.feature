Feature: down

  Rule: down drains every record the last check found fixed, records each new in-source ignore as suppressed, and nothing else

    @succeeds
    Scenario Outline: down succeeds
      Given the down input "<input>"
      When down runs
      Then down answers "<expected>"

      Examples:
        | name    | input   | expected   |
        | fixed   | fixed   | 1 dropped  |
        | nothing | nothing | 0 dropped  |
        | ignored | ignored | 1 dropped, recorded src/b.rs suppressed |

    @refuses
    Scenario Outline: down refuses
      Given the down input "<input>"
      When down runs
      Then down answers "<expected>"

      Examples:
        | name      | input     | expected                                  |
        | unchecked | unchecked | no check has run here with this asymptote |

  Rule: down drops a perf record whose tool ran and no longer reports it, in one operation

    @succeeds
    Scenario Outline: down drains perf records
      When down drains the measured product "<input>"
      Then down answers "<expected>"

      Examples:
        | input       | expected |
        | resolved    | 1 perf dropped; best -; operations 2 |
        | held        | 0 perf dropped; best 100; operations 1 |
        | other-group | 0 perf dropped; best 100; operations 1 |
