Feature: verify_rule_profiles
  @succeeds
  Scenario: A rule with its own settings profile is verified under it
    Then verify_rule_profiles gives each rule its own staged profile

  @succeeds
  Scenario: A bench without rule profiles is verified whole
    Then verify_rule_profiles gives a bench without rule profiles to the verification once
