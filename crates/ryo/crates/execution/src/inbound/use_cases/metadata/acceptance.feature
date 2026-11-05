Feature: metadata

  Rule: metadata answers what its caller sees

    @succeeds
    Scenario Outline: metadata succeeds
      Given <input>
      When metadata runs
      Then <expected> is answered

      Examples:
        | name | input | expected |
