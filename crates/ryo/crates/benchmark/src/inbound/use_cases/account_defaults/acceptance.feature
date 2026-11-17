Feature: account_defaults
  @succeeds
  Scenario: An authored default is accounted for
    Then account_defaults accepts an audit that authored the default

  @refuses
  Scenario: An audit without authored rules is refused
    Then account_defaults refuses an audit that authored nothing

  @refuses
  Scenario: A default listed as deferred and also authored is refused
    Then account_defaults refuses a tool whose deferred default eslint.no-with is also authored
