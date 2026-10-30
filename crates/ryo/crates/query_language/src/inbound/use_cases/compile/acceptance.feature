Feature: compile

  Rule: compile answers what its caller sees

    @succeeds
    Scenario Outline: compile succeeds
      Given the compile input "<input>"
      When compile runs
      Then compile answers "<expected>"

      Examples:
        | name | input | expected |
        | valid | valid | 1 steps |

    @refuses
    Scenario Outline: compile refuses
      Given the compile input "<input>"
      When compile runs
      Then compile answers "<expected>"

      Examples:
        | name | input | expected |
        | unknown_relation | unknown_relation | no relation named `nothing` |
