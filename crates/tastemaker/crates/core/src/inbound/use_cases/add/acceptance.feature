Feature: add

  Rule: add answers what its caller sees

    @succeeds
    Scenario Outline: add succeeds
      Given the add input "<input>"
      When add runs
      Then add answers "<expected>"

      Examples:
        | name | input | expected |
        | spec-dry-run | part | p/part@0.1.0 |
        | persisted-entry | write | entry and lock written |

    @refuses
    Scenario Outline: add refuses
      Given the add input "<input>"
      When add runs
      Then add answers "<expected>"

      Examples:
        | name | input | expected |
        | no-definition | absent | no package of the workspace holds `absent` |
