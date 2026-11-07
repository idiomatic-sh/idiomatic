Feature: select_cases
  @succeeds
  Scenario: A shard flag chooses its coordinates
    Then select_cases reads the shard a flag names

  @refuses
  Scenario: A repeated shard flag is refused
    Then select_cases refuses a repeated shard flag
