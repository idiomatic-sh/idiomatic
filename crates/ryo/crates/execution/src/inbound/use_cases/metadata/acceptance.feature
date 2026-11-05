Feature: metadata

  Rule: Pure metadata decisions expose checked request facts

    @succeeds
    Scenario: The feature catalog is available
      Given a metadata request "catalog"
      When metadata runs
      Then metadata answers "catalogued"

    @refuses
    Scenario: A foreign affected file is refused
      Given a metadata request "foreign file"
      When metadata runs
      Then metadata answers "refused"
