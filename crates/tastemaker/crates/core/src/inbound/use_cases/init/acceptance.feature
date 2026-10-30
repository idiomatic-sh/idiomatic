Feature: init
  Rule: initialize local assets without external packages
    @succeeds
    Scenario Outline: initialization plans or writes
      Given the init input "<input>"
      When init runs
      Then init answers "<expected>"
      Examples:
        | name | input | expected |
        | fresh | fresh | local package initialized |
        | preview | preview | no files written |
    @refuses
    Scenario Outline: initialization preserves existing files
      Given the init input "<input>"
      When init runs
      Then init answers "<expected>"
      Examples:
        | name | input | expected |
        | rerun | rerun | existing files preserved |
        | conflict | conflict | existing files preserved |
    @succeeds
    Scenario: a project authors and enforces its own assets
      Given the init input "fresh"
      When init runs
      Then the initialized project can author and enforce its own rule
