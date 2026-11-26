Feature: parse

  Rule: parse answers what the language says of its files

    @refuses
    Scenario Outline: parse refuses
      Given the parse input "<input>"
      When parse runs
      Then parse answers "<expected>"

      Examples:
        | name             | input                   | expected                             |
        | unsupported_file | contract.not-a-language | the language does not read this file |

    @succeeds
    Scenario Outline: parse says what the language is
      Given the parse input "<input>"
      When parse runs
      Then parse answers "<expected>"

      Examples:
        | name       | input          | expected |
        | no_kinds   | contract.kinds | 0 kinds  |
