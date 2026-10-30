Feature: scaffold

  Rule: scaffold answers what its caller sees

    @succeeds
    Scenario Outline: scaffold succeeds
      Given the scaffold input "<input>"
      When scaffold runs
      Then scaffold answers "<expected>"

      Examples:
        | name | input | expected |
        | missing-file | src/b | wrote src/b/errors.txt |

    @refuses
    Scenario Outline: scaffold refuses
      Given the scaffold input "<input>"
      When scaffold runs
      Then scaffold answers "<expected>"

      Examples:
        | name | input | expected |
        | no-entry | absent | no entry `absent` in the workspace's tastemaker.toml files |

  Rule: scaffold settles a real prospective tree before writing

    @succeeds
    Scenario: a required template waits for its generated sibling
      Given the scaffold input "disk-sibling"
      When scaffold runs
      Then scaffold answers "dependent written"

    @refuses
    Scenario: a condition cycle publishes no file
      Given the scaffold input "disk-cycle"
      When scaffold runs
      Then scaffold answers "unsettled, no files written"

    @refuses
    Scenario: unresolved condition parameters publish no file
      Given the scaffold input "disk-unresolved"
      When scaffold runs
      Then scaffold answers "undecided, no files written"
