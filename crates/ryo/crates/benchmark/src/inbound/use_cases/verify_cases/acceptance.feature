Feature: verify_cases
  @succeeds
  Scenario: Independent authored cases execute
    Then verify_cases answers two independently authored cases
  @refuses
  Scenario: Missing input refuses before execution
    Then verify_cases refuses missing input before a process launches
