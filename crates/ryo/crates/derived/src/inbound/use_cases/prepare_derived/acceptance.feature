Feature: prepare_derived

  Rule: preparation checks a request against the schemas of its inputs

    @succeeds
    Scenario: a request on a present column keeps its schema
      Given the prepare_derived input "present column"
      When prepare_derived runs
      Then prepare_derived answers "prepared 1"

    @refuses
    Scenario: a request on an absent column is refused on validation
      Given the prepare_derived input "absent column"
      When prepare_derived runs
      Then prepare_derived answers "no such column 3"
