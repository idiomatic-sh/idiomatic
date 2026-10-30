Feature: open

  Rule: open answers what its caller sees

    @succeeds
    Scenario Outline: open succeeds
      Given the open input "<input>"
      When open runs
      Then open answers "<expected>"

      Examples:
        | name | input | expected |
        | small | 1024 | opened . |

    @refuses
    Scenario Outline: open refuses
      Given the open input "<input>"
      When open runs
      Then open answers "<expected>"

      Examples:
        | name | input | expected |
        | too_big | 18446744073709551615 | memory: 18446744073709551615 bytes asked for, 1073741824 available |
