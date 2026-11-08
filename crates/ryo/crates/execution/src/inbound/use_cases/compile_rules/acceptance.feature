Feature: compile_rules

  Rule: Rules are compiled only when their ids, queries and bindings are sound

    @succeeds
    Scenario: Distinct rules compile
      Given compile rules "distinct"
      When compilation runs
      Then compilation answers "compiled"

    @refuses
    Scenario: A repeated rule id is refused
      Given compile rules "repeated"
      When compilation runs
      Then compilation answers "refused"
