Feature: list

  Rule: list answers what its caller sees

    @succeeds
    Scenario Outline: list succeeds
      Given the list input "<input>"
      When list runs
      Then list answers "<expected>"

      Examples:
        | name | input | expected |
        | every | all | marker part |

    @refuses
    Scenario Outline: list refuses
      Given the list input "<input>"
      When list runs
      Then list answers "<expected>"

      Examples:
        | name | input | expected |
        | nowhere | nowhere | no tastemaker.toml at or above `/fake` |
