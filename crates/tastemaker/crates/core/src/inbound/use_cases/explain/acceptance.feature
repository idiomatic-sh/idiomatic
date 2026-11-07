Feature: explain

  Rule: explain answers what its caller sees

    @succeeds
    Scenario Outline: explain succeeds
      Given the explain input "<input>"
      When explain runs
      Then explain answers "<expected>"

      Examples:
        | name | input | expected |
        | topic | selectors | selectors |

    @refuses
    Scenario Outline: explain refuses
      Given the explain input "<input>"
      When explain runs
      Then explain answers "<expected>"

      Examples:
        | name | input | expected |
        | unknown | nothing | `nothing` is no path, no `<kind>/<name>` and no topic |
