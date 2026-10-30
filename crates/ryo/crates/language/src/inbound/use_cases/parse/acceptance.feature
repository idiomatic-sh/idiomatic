Feature: parse

  Rule: parse answers what its caller sees

    @refuses
    Scenario Outline: parse refuses
      Given the parse input "<input>"
      When parse runs
      Then parse answers "<expected>"

      Examples:
        | name | input | expected |
        | unsupported_file | contract.not-a-language | the language does not read this file |
