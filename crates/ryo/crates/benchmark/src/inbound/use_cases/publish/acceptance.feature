Feature: publish
  @succeeds
  Scenario: A measured complete inventory permits metric publication
    Then the complete certified inventory publishes its observed samples

  @refuses
  Scenario: A diagnostic pair cannot become benchmark metrics
    Then the diagnostic certified subset refuses metric publication
