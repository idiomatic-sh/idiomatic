Feature: collect
  @succeeds
  Scenario: A child launch error crosses the protocol as an observation
    Then the collector preserves a missing executable error

  @refuses
  Scenario: A malformed request never launches
    Then the collector refuses malformed input before execution
