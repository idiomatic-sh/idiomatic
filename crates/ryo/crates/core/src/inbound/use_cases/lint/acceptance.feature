Feature: lint

  Rule: lint answers what its caller sees

    @succeeds
    Scenario Outline: lint succeeds
      Given the lint input "<input>"
      When lint runs
      Then lint answers "<expected>"

      Examples:
        | name | input | expected |
        | two_violations | two_rust_files_and_a_text_file | 2 findings and 1 unread |
        | clean_text_only | only_a_text_file | 0 findings and 1 unread |

    @refuses
    Scenario Outline: lint refuses
      Given the lint input "<input>"
      When lint runs
      Then lint answers "<expected>"

      Examples:
        | name | input | expected |
        | refused_rule | refused_text | rule `rule`: the fake refuses text that contains `refuse` |
        | corpus_down | corpus_down | corpus: cannot read `.`: the fake was told to fail |
