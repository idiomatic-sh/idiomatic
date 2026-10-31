Feature: list_operators

  Rule: list_operators answers what its caller sees

    @succeeds
    Scenario Outline: list_operators succeeds
      Given the list_operators input "<input>"
      When list_operators runs
      Then list_operators answers "<expected>"

      Examples:
        | name | input | expected |
        | all | none | 19 operators |
