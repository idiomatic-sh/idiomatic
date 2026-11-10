Feature: operators

  Rule: operators preserve borrowed row semantics

    @succeeds
    Scenario: project repeats selected columns in plan order
      Given the operators input "duplicate columns"
      When operators runs
      Then operators answers "9 7 9"

    @refuses
    Scenario: project reports its first absent column
      Given the operators input "missing column"
      When operators runs
      Then operators answers "missing 3"
