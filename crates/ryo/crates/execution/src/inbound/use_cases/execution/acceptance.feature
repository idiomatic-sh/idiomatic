Feature: execution

  Rule: A request execution begins over a prepared program and finishes

    @succeeds
    Scenario: An empty request begins and finishes
      Given an execution request "empty"
      When execution runs
      Then execution answers "complete"
