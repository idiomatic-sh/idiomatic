# repeated-string-literals

## 0.2.0

An import specifier is not a repeated literal. 0.1.0 read every `string-literal` row, so
`'../../context'`, the specifier of `import { x } from '../../context'` written in many
files, was reported as the same text written in many files. A literal that names a module
where it is loaded (`import .. from`, `export .. from`, `import()`, `require()`,
`import_module()`, an `import` path, a `#[path]`, an `@import`) has its `import` column
raised, and the query keeps only the rows with `import = "false"`.

### Moving up from 0.1.0

Set `version = "0.2.0"` on the entry. An `allowed` value or an `exclude` that only kept
import specifiers out is no longer needed.

### Fixtures

0.1.0's three, and `import-specifier-three-files` (passing): one module imported by the
same specifier in three files is no finding.
