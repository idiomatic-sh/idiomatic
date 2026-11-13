# no-negated-feature-cfg 0.1.0

New rule: code compiled only when a feature is off, `not` over a `feature` in a `cfg` or `cfg_attr` attribute or in a `cfg!` call, is a finding. Fixtures `additive-features` (pass: a feature, the `any(test, feature = "fixtures")` seal, `not(test)`), `negated-feature-attributes`, `negated-feature-macro` and `the-spike-evaluator-flag` (violations: the fused-evaluation spike's three interpreter arms).
