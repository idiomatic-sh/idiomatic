Feature: operate
  @succeeds
  Scenario: A language bench passes when every default passes and passes only the worker's document through
    Then a bench writes exactly what its sealed worker wrote

  @refuses
  Scenario: A bench whose cases fail measures nothing
    Then a bench whose verification fails runs no corpus

  @refuses
  Scenario: An absent reference tool is named before anything is built
    Then an operation without its reference tool is refused by the tool's name
