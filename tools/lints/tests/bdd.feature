Feature: Compiler-backed nominal interface boundaries

  Scenario: A cross-crate implementation supplies an owned domain type
    When the implementation supplies a nominal type
    Then native checking accepts the implementation

  Scenario: An alias cannot disguise a primitive boundary
    When the implementation supplies a foreign primitive alias
    Then native checking rejects the representation

  Scenario: An unresolved implementation cannot appear verified
    When the implementation leaves its boundary type unresolved
    Then native checking reports an unproven boundary

  Scenario: Unrelated Rust traits retain their own contracts
    When an unrelated Rust trait exposes a primitive
    Then native checking accepts the implementation

  Scenario: A generic implementation retains nominal ownership
    When a generic implementation is restricted to nominal owners
    Then native checking accepts the implementation

  Scenario: An owner cannot admit primitives through a generic bound
    When the bound owner also admits a primitive
    Then native checking reports an unproven boundary
