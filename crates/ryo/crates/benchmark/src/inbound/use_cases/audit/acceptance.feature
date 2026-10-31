Feature: audit
  @succeeds
  Scenario: All selected defaults have implementations
    Then complete default coverage permits measurement

  @refuses
  Scenario: A missing default cannot silently disappear
    Then missing default coverage prevents measurement
