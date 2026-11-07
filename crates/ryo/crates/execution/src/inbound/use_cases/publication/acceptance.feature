Feature: publication

  Rule: Producer names become portable before their symbols are released

    @succeeds
    Scenario Outline: Complete producer values remain readable
      Given a producer row "<input>"
      When publication runs
      Then publication answers "<expected>"

      Examples:
        | input | expected |
        | numeric | portable numeric |
        | symbol | portable symbol |

    @refuses
    Scenario: A reserved arena cannot publish
      Given a producer row "reserved"
      When publication runs
      Then publication answers "refused unchanged"
