Feature: view

  Rule: a view borrows what the payload of the request's kind holds

    @succeeds
    Scenario: an equality payload is viewed as its index
      Given the view input "equality payload"
      When view runs
      Then view answers "index of 2 rows"

    @refuses
    Scenario: a payload of another kind is the wrong operation
      Given the view input "raw order payload"
      When view runs
      Then view answers "wrong operation"
