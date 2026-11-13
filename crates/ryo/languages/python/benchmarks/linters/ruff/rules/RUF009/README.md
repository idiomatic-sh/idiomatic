RUF009 candidate for pinned Ruff 0.16.5 default policy, Python target 3.10.

The query checks annotated direct class-body defaults whose expression is a call. It uses the first recognized stdlib/attrs dataclass decorator, follows the pinned immutable annotation and immutable function catalogs, and distinguishes field calls by dataclass kind. It resolves local descriptor, frozen dataclass and immutable NewType definitions through existing semantic bindings. The attrs auto_attribs setting and final typing ClassVar binding are checked from source facts. Standard suppression applies to the called expression.

Source contract: Ruff 0.16.5 `function_call_in_dataclass_default.rs`, `helpers.rs`, `ruff_python_semantic/src/analyze/typing.rs`, and `ruff_python_stdlib/src/typing.rs`. Complete tagged fixture bytes are copied under `upstream/`:
- `RUF009.py` SHA-256 `ae8d3066398860fc355834103edb11603864b57e8dbbcba2b1ea86fcf853747a`
- `RUF009_attrs.py` SHA-256 `2807da974586f4815069423e2f7394d8ac5ec3b72f80ba85bf808508200486e4`
- `RUF009_attrs_auto_attribs.py` SHA-256 `e12cb0a56c8427f963a60dc73b41dd532e7de5440a650682d80ad30a77fca55b`
- `RUF009_deferred.py` SHA-256 `14cf24d20360749361c34dd413a6287da32a7a47ae81d81457b53e0e9a80d136`

The cases file retains the complete output of all four tagged fixtures and independent controls for decorator mode, stdlib/attrs field separation, descriptor methods, frozen classes, NewType mutability, ClassVar shadowing, unknown auto_attribs, and noqa. Source presence alone is not admission; the exact current-helper stage, worker and raw receipts must pass before the rule enters the public PASS count.
