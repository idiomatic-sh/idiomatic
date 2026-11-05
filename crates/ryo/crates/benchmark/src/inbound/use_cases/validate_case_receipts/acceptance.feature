Feature: Validate sealed receipt-only registrations
  @succeeds
  Scenario: Shared producer registration verifies both recorded sides
    Then receipt validation admits the shared producer registration

  @refuses
  Scenario: Missing authority refuses admission
    Then receipt validation refuses a missing authority

  @refuses
  Scenario: Missing module inventory refuses admission
    Then receipt validation refuses a missing module inventory

  @refuses
  Scenario: Missing artifact inventory refuses admission
    Then receipt validation refuses a missing artifact inventory
