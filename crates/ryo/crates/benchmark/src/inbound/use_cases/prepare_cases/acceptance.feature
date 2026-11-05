Feature: prepare_cases

  @succeeds
  Scenario: Complete through the public operation
    Then prepare_cases complete preserves its promised result

  @refuses
  Scenario: Missing through the public operation
    Then prepare_cases missing preserves its promised result
