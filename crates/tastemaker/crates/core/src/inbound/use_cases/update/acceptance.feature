Feature: update

  Rule: update answers what its caller sees

    @succeeds
    Scenario Outline: update succeeds
      Given the update input "<input>"
      When update runs
      Then update answers "<expected>"

      Examples:
        | name | input | expected |
        | latest | every | marker 0.1.0>0.2.0 |
        | persisted-upgrade | write | lock uses 0.2.0 |
        | dry-run | dry-run | lock unchanged |
        | transitive-and-members | transitive | transitive releases and selected locals locked |

    @refuses
    Scenario Outline: update refuses
      Given the update input "<input>"
      When update runs
      Then update answers "<expected>"

      Examples:
        | name | input | expected |
        | no-entry | absent | no entry `absent` in the root tastemaker.toml |
        | absent-target | missing-target | entry `marker` has no offered version `99.0.0` |
        | unresolved-import | missing-transitive | unresolved transitive asset refused before writes |

    @fails
    Scenario Outline: update reports partial writes
      Given the update input "<input>"
      When update runs
      Then update answers "<expected>"

      Examples:
        | name | input | expected |
        | lock-write | lock-write | lock write failed; check --fix repairs |
