Feature: compare
  @succeeds
  Scenario: Finding order does not change agreement
    Then equivalent diagnostic multisets agree

  @refuses
  Scenario: A duplicate finding is significant
    Then an extra diagnostic occurrence prevents agreement
