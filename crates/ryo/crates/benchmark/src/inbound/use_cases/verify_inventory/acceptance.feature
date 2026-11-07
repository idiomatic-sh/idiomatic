Feature: verify_inventory
  @succeeds
  Scenario: Missing rules remain failures in the complete inventory
    Then the inventory records a missing rule and completes

  @refuses
  Scenario: Empty defaults cannot pass as complete coverage
    Then the inventory refuses empty defaults before execution
