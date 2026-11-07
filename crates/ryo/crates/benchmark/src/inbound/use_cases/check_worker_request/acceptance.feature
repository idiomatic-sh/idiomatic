Feature: check_worker_request
  @succeeds
  Scenario: The executable's own target and phases are accepted
    Then check_worker_request accepts the request carrying the expected identity

  @refuses
  Scenario: Another target than the executable's is refused
    Then check_worker_request refuses a request retargeted to another version
