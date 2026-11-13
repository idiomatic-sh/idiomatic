Feature: operate
  @succeeds
  Scenario: Parity passes only the worker's metrics through
    Then a parity operation writes exactly what its sealed worker wrote

  @refuses
  Scenario: An absent reference tool is named before anything is built
    Then an operation without its reference tool is refused by the tool's name
