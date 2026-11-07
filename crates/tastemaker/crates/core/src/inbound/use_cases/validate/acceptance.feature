Feature: validate

  Rule: validate answers what its caller sees

    @succeeds
    Scenario Outline: validate succeeds
      Given the validate input "<input>"
      When validate runs
      Then validate answers "<expected>"

      Examples:
        | name | input | expected |
        | proven | p | 1 proven |

    @refuses
    Scenario Outline: validate refuses
      Given the validate input "<input>"
      When validate runs
      Then validate answers "<expected>"

      Examples:
        | name | input | expected |
        | missing-package | q | package `q`: no directory at `/q` |

  Rule: explicit filters select versions from a real package

    @succeeds
    Scenario: a matching filter proves the on-disk template
      Given the validate input "disk-match"
      When validate runs
      Then validate answers "1 proven"

    @refuses
    Scenario: a misspelled filter cannot certify an empty validation
      Given the validate input "disk-unmatched"
      When validate runs
      Then validate answers "validation selection matched no definition versions"
