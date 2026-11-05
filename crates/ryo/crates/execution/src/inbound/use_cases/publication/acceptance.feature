Feature: Portable producer publication
  Scenario: Numeric rows retain their values without allocating name storage
    Given a complete numeric producer row
    When the publication is normalized
    Then the row and prior arena bytes are unchanged
  Scenario: Reserved worker identifiers cannot name portable arena storage
    Given the reserved cross-file worker identifier
    When the publication is normalized
    Then publication is refused before mutating its output
