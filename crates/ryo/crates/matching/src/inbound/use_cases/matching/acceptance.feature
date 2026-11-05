Feature: matching

  Rule: Semantic matching answers without execution state

    @succeeds
    Scenario Outline: Matching compiled values
      Given the matching input "<input>"
      When matching runs
      Then matching answers "<expected>"

      Examples:
        | input | expected |
        | regex-positive | match |
        | regex-negative | no match |
        | glob-separator | 1 |

    @refuses
    Scenario Outline: Invalid patterns retain their semantic refusal
      Given the matching input "<input>"
      When matching runs
      Then matching answers "<expected>"

      Examples:
        | input | expected |
        | invalid-regex | invalid regex |
        | invalid-glob | invalid glob |
