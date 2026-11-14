Feature: plan

  Rule: a program is planned unless its shape is not a program

    @succeeds
    Scenario: a program whose nodes read only earlier nodes is planned
      Given the plan program "reads earlier"
      When plan runs
      Then plan answers "planned"

    @refuses
    Scenario: a node that reads a later node is refused
      Given the plan program "reads later"
      When plan runs
      Then plan answers "refused WrongOperation"
