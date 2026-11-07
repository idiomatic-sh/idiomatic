Feature: create

  Rule: create answers what its caller sees

    @succeeds
    Scenario Outline: create succeeds
      Given the create input "<input>"
      When create runs
      Then create answers "<expected>"

      Examples:
        | name | input | expected |
        | rule | rule/no-tsv --set extension=rs | rules/no-tsv/0.1.0/rule.toml |

    @refuses
    Scenario Outline: create refuses
      Given the create input "<input>"
      When create runs
      Then create answers "<expected>"

      Examples:
        | name | input | expected |
        | no-package | spec/x | the workspace names no package by path: pass `--package <dir>` |
        | no-extension | rule/no-tsv | `rule/no-tsv` needs the extension its scenario files end in |
