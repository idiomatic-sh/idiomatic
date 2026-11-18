Feature: operate
  @succeeds
  Scenario: A language bench passes when every default passes and writes only the deterministic measures of the worker's document
    Then a bench writes the deterministic measures of the document its sealed worker wrote, as one BMF document

  @refuses
  Scenario: A bench whose cases fail measures nothing
    Then a bench whose verification fails runs no corpus

  @refuses
  Scenario: An absent reference tool is named before anything is built
    Then an operation without its reference tool is refused by the tool's name
