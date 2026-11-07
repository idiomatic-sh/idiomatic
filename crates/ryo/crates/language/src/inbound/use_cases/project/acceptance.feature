Feature: project

  Rule: project answers what its caller sees

    @succeeds
    Scenario Outline: project succeeds
      Given the project input "<input>"
      When project runs
      Then project answers "<expected>"

      Examples:
        | name | input | expected |
        | no_manifests | none | 0 rows |
