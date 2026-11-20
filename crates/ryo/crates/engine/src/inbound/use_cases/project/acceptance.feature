Feature: project

  Rule: project runs the cross-file rules once over the frontier rows

    @succeeds
    Scenario: an empty rule set's project pass finds nothing
      Given the project input "no rules"
      When project runs
      Then project found "0" rows

    @succeeds
    Scenario: a frontier row emitted by a file reaches the project pass
      Given the project input "frontier round trip"
      When project runs
      Then project found "1" rows
