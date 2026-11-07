Feature: build

  Rule: a build answers the payload of the prepared request or refuses inputs of another kind

    @succeeds
    Scenario: an equality request builds a payload
      Given the build input "equality rows"
      When build runs
      Then build answers "built"

    @refuses
    Scenario: inputs of another kind are the wrong operation
      Given the build input "raw words"
      When build runs
      Then build answers "wrong operation"
