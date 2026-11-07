Feature: prepare

  Rule: preparation checks a request against the schemas of its inputs

    @succeeds
    Scenario: a request on a present column keeps its schema
      Given the prepare input "present column"
      When prepare runs
      Then prepare answers "prepared 1"

    @refuses
    Scenario: a request on an absent column is refused on validation
      Given the prepare input "absent column"
      When prepare runs
      Then prepare answers "no such column 3"
