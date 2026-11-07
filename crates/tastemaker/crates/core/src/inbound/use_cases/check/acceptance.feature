Feature: check

  Rule: persisted source inventory preserves check behavior

    @succeeds
    Scenario: cold, reopened warm and edited checks use the production store
      Given a real consumer whose referenced rule has an incorrect case
      When the referenced rule case is corrected
      And the real consumer validates
      And the real consumer checks through a reopened and edited store
      Then check answers "persistent lifecycle holds"

  Rule: real package proofs hold the consumer check

    @refuses
    Scenario: removing a dependency invalidates a warm consumer plan
      Given a real consumer whose referenced rule has an incorrect case
      When the referenced rule case is corrected
      And the real consumer validates
      And check runs
      Then check answers "clean"
      When the referenced package dependency is removed
      And check runs
      Then check answers "unresolved dependency"

    @succeeds
    Scenario: concurrent builds leave one store, which each build discards and proves again
      Given a real consumer whose referenced rule has an incorrect case
      When the referenced rule case is corrected
      And two builds validate concurrently and each validates and checks the consumer
      Then check answers "both builds clean"

    @succeeds
    Scenario Outline: only current successful proofs permit a consumer check
      Given a real consumer whose referenced rule has an incorrect case
      When check runs
      Then check answers "proof refused"
      When the real consumer validates
      Then check answers "failed"
      When check runs
      Then check answers "proof refused"
      When the referenced rule case is corrected
      And check runs
      Then check answers "proof refused"
      When the real consumer validates
      Then check answers "proven"
      When check runs
      Then check answers "clean"
      When the proof input "<path>" is added
      And check runs
      Then check answers "proof refused"
      When the real consumer validates
      Then check answers "proven"
      When check runs
      Then check answers "clean"
      When the proof input "<path>" is deleted
      And check runs
      Then check answers "proof refused"
      When the real consumer validates
      Then check answers "proven"
      When check runs
      Then check answers "clean"

      Examples:
        | kind | path |
        | referenced rule | a/rules/marker/0.1.0/new-input.txt |
        | spec | p/specs/part/0.1.0/new-input.txt |
        | template | p/templates/module/0.1.0/new-input.txt |
        | fixture-only dependency | p/templates/support/0.1.0/new-input.txt |

  Rule: check answers what its caller sees

    @succeeds
    Scenario Outline: check succeeds
      Given the check input "<input>"
      When check runs
      Then check answers "<expected>"

      Examples:
        | name | input | expected |
        | placed | placed | p/marker p/part#closed p/part#closed p/part/errors#count |
        | ignored | ignored | p/marker p/marker p/part#closed p/part#closed p/part/errors#count tastemaker#ignore, suppressed 1 |
        | dry-run | dry-run | Planned, lock written false |
        | unsaved store | unsaved | p/marker p/part#closed p/part#closed p/part/errors#count |

    @refuses
    Scenario Outline: check refuses
      Given the check input "<input>"
      When check runs
      Then check answers "<expected>"

      Examples:
        | name | input | expected |
        | nowhere | nowhere | no tastemaker.toml at or above `/fake` |
        | missing proof | unproven | `p/rule/marker@0.1.0` has no current successful proof (missing or stale validation); run `tm validate` |
