Feature: verify_reference_cases
  @succeeds
  Scenario: Independent authored cases execute
    Then verify_reference_cases answers two independently authored cases
  @refuses
  Scenario: Missing input refuses before execution
    Then verify_reference_cases refuses missing input before a process launches
