Feature: typecheck

  Rule: typecheck answers what its caller sees

    @succeeds
    Scenario Outline: typecheck succeeds
      Given the typecheck input "<input>"
      When typecheck runs
      Then typecheck answers "<expected>"

      Examples:
        | name | input | expected |
        | valid | valid | reads 1 relations |

    @refuses
    Scenario Outline: typecheck refuses
      Given the typecheck input "<input>"
      When typecheck runs
      Then typecheck answers "<expected>"

      Examples:
        | name | input | expected |
        | unknown_relation | unknown_relation | no relation named `nothing` (from `<query>` at byte 0) |
