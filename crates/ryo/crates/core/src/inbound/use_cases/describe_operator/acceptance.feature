Feature: describe_operator

  Rule: describe_operator answers what its caller sees

    @succeeds
    Scenario Outline: describe_operator succeeds
      Given the describe_operator input "<input>"
      When describe_operator runs
      Then describe_operator answers "<expected>"

      Examples:
        | name | input | expected |
        | antijoin | antijoin | 4 fields |

    @refuses
    Scenario Outline: describe_operator refuses
      Given the describe_operator input "<input>"
      When describe_operator runs
      Then describe_operator answers "<expected>"

      Examples:
        | name | input | expected |
        | unknown | sort | no operator named `sort` |
