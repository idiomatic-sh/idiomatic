Feature: describe_relation

  Rule: describe_relation answers what its caller sees

    @succeeds
    Scenario Outline: describe_relation succeeds
      Given the describe_relation input "<input>"
      When describe_relation runs
      Then describe_relation answers "<expected>"

      Examples:
        | name | input | expected |
        | function | function | 4 columns |

    @refuses
    Scenario Outline: describe_relation refuses
      Given the describe_relation input "<input>"
      When describe_relation runs
      Then describe_relation answers "<expected>"

      Examples:
        | name | input | expected |
        | unknown | nothing | no relation named `nothing` |
