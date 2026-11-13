Feature: prepare

  Rule: A request is prepared only when its plans, bindings and rules agree

    @succeeds
    Scenario: An empty request is prepared
      Given a prepare request "empty"
      When preparation runs
      Then preparation answers "prepared"

    @refuses
    Scenario: A binding without its rule is refused
      Given a prepare request "mismatched bindings"
      When preparation runs
      Then preparation answers "refused"
