Feature: execution

  Rule: A request owns preparation and completion

    @succeeds
    Scenario: An empty request begins and finishes
      Given an execution request "empty"
      When execution runs
      Then execution answers "complete"

    @refuses
    Scenario: A binding without its rule is refused
      Given an execution request "mismatched bindings"
      When execution runs
      Then execution answers "refused"
