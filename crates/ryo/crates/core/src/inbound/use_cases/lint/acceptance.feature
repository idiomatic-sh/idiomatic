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
        | new_engine_runs_the_rule | the_new_engine_runs_the_rule | 6 findings and 0 unread |
        | new_engine_refuses_the_rule | the_new_engine_refuses_the_rule | 0 findings and 0 unread; `rule` not decided at `.` |

    @refuses
    Scenario Outline: lint refuses
      Given the lint input "<input>"
      When lint runs
      Then lint answers "<expected>"

      Examples:
        | name | input | expected |
        | refused_rule | refused_text | rule `rule`: the fake refuses text that contains `refuse` |
        | corpus_down | corpus_down | corpus: cannot read `.`: the fake was told to fail |

  Rule: a rule that passes its allowance is warned of and never stopped

    @succeeds
    Scenario: in a user's run an overrun is a line of the report and nothing else changes
      Given the lint input "the_new_engine_notices_an_overrun"
      When lint runs
      Then lint answers "6 findings and 0 unread"
      And lint warns
        """
        warning: rule `rule` is slow in `b.walk`
          statement `pairs` (rule.ryo:3) joins `words` (1000 rows) with `words` (1000 rows) with no key between them
          that is 1000000 pairs in this one file; the run continues and its findings are complete
          to fix: join on a shared column or add a range, so each row meets only its matches
        """
