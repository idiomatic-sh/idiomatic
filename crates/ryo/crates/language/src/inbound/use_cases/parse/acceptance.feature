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

  Rule: parse keeps what the language's read gives of the file

    @succeeds
    Scenario Outline: parse keeps the flow facts the read gave
      Given the parse input "<input>"
      When parse runs
      Then parse answers "<expected>"

      Examples:
        | name            | input                   | expected                    |
        | kept_graph      | contract.flow-two-block | read, 2 flow blocks, 1 edge |
        | discarded_graph | contract.flow-discarded | read, 0 flow blocks         |
