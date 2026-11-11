# test-group-module 0.2.0

A group module need not declare a `test` of its own: a module that only declares its nested groups is a group namespace. 0.1.0's content required `pub(super) mod test;`, so a namespace failed. The template no longer requires any declaration; the shape is held by the rule `test-group-namespace`, which the placing spec applies: a group module declares modules only, never a test function or a helper. Scenario `group-namespace-module`.
