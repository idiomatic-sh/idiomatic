Feature: validate

  Rule: validate answers what its caller sees

    @succeeds
    Scenario Outline: validate succeeds
      Given the validate input "<input>"
      When validate runs
      Then validate answers "<expected>"

      Examples:
        | name | input | expected |
        | valid query | valid | valid, reading [] |
        | hosted relation validated with its declaration | hosted | valid, reading [host.placed] |
        | no languages linked, kinds not asked | kind, no language | valid, reading [] |

    @refuses
    Scenario Outline: validate refuses
      Given the validate input "<input>"
      When validate runs
      Then validate answers "<expected>"

      Examples:
        | name | input | expected |
        | syntax error | refuse | the fake refuses text that contains `refuse` |
        | unknown kind refused | kind | kind `no-such-kind` names no kind of fake |
