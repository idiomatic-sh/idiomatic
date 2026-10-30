Feature: describe

  Rule: describe answers what its caller sees

    @succeeds
    Scenario Outline: describe succeeds
      Given the describe input "<input>"
      When describe runs
      Then describe answers "<expected>"

      Examples:
        | name | input | expected |
        | grammar | none | starts at program |
