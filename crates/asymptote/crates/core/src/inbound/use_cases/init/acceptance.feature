Feature: init

  Rule: init writes a configuration where there is none and one script for each hook it lists

    @succeeds
    Scenario Outline: init succeeds
      Given the init input "<input>"
      When init runs
      Then init answers "<expected>"

      Examples:
        | input            | expected |
        | fresh            | Created; installed pre-commit; removed -; scripts .git/hooks/pre-commit; manifest yes; operations 1 |
        | rerun            | Kept; installed pre-commit; removed -; scripts .git/hooks/pre-commit; manifest yes; operations 1 |
        | removed-hook     | Kept; installed pre-push; removed pre-commit; scripts .git/hooks/pre-push; manifest yes; operations 1 |
        | existing         | Kept; installed pre-commit; removed -; scripts .git/hooks/pre-commit; manifest yes; operations 0 |
        | dry-run          | Created; installed pre-commit; removed -; scripts -; manifest no; operations 0 |
        | hooks-path       | Created; installed pre-commit; removed -; scripts hooks-here/pre-commit; manifest yes; operations 1 |
        | foreign-unlisted | Created; installed pre-commit; removed -; scripts .git/hooks/commit-msg,.git/hooks/pre-commit; manifest yes; operations 1 |

  Rule: init never overwrites a hook it does not own, and writes nothing when it refuses

    @refuses
    Scenario Outline: init refuses
      Given the init input "<input>"
      When init runs
      Then init answers "<expected>"

      Examples:
        | input        | expected |
        | foreign      | `$ROOT/.git/hooks/pre-commit` is the `pre-commit` hook, and asymptote did not write it: init never overwrites a hook it does not own; scripts .git/hooks/pre-commit; manifest no; operations 0 |
        | not-a-repo   | `$ROOT` is in no git repository; scripts -; manifest no; operations 0 |
