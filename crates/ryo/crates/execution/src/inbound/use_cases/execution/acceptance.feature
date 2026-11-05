Feature: execution

  Rule: execution answers what its caller sees

    @succeeds
    Scenario Outline: execution succeeds
      Given <input>
      When execution runs
      Then <expected> is answered

      Examples:
        | name | input | expected |
