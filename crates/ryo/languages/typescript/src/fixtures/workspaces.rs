//! Small pnpm, Bun, npm and Yarn workspaces and repositories: members, catalogs, unnamed packages, peers and the references to them.

/// A pnpm workspace: members from `pnpm-workspace.yaml`, one excluded by negation.
pub(crate) const PNPM_MEMBERS: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    ("packages/a/package.json", r#"{"name":"a"}"#),
    ("packages/skip/package.json", r#"{"name":"skip"}"#),
    (
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\n  - '!packages/skip'\n",
    ),
];

/// The same repository without its workspace file: nothing declares members.
pub(crate) const PNPM_NO_WORKSPACE_FILE: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    ("packages/a/package.json", r#"{"name":"a"}"#),
    ("packages/skip/package.json", r#"{"name":"skip"}"#),
];

/// A pnpm default catalog whose entry names another package.
pub(crate) const PNPM_DEFAULT_CATALOG: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"catalog:"}}"#,
    ),
    ("packages/real/package.json", r#"{"name":"real"}"#),
    (
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\ncatalog:\n  core: npm:real@^1\n",
    ),
];

/// A pnpm catalog entry that is a path: the path is from the workspace file, not from the package that uses the catalog.
pub(crate) const PNPM_PATH_CATALOG: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"lib":"catalog:"}}"#,
    ),
    ("packages/lib/package.json", r#"{"name":"lib"}"#),
    (
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\ncatalog:\n  lib: file:./packages/lib\n",
    ),
];

/// The default catalog writes a plain range: the dependency stays on its own name.
pub(crate) const PNPM_PLAIN_CATALOG: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"catalog:"}}"#,
    ),
    ("packages/core/package.json", r#"{"name":"core"}"#),
    (
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\ncatalog:\n  core: ^1\n",
    ),
];

/// A pnpm named catalog, referenced by `catalog:legacy`.
pub(crate) const PNPM_NAMED_CATALOG: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"catalog:legacy"}}"#,
    ),
    ("packages/real/package.json", r#"{"name":"real"}"#),
    (
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\ncatalogs:\n  legacy:\n    core: npm:real@^1\n",
    ),
];

/// The package asks for the default catalog, and only a named catalog exists.
pub(crate) const PNPM_NAMED_ONLY: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"catalog:"}}"#,
    ),
    ("packages/core/package.json", r#"{"name":"core"}"#),
    (
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\ncatalogs:\n  legacy:\n    core: ^1\n",
    ),
];

/// A `catalog:` reference to an entry no catalog has.
pub(crate) const MISSING_ENTRY: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"catalog:"}}"#,
    ),
    ("packages/core/package.json", r#"{"name":"core"}"#),
    (
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\ncatalog:\n  other: ^1\n",
    ),
];

/// A Bun catalog beside the workspace packages.
pub(crate) const BUN_WORKSPACES_CATALOG: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"root","workspaces":{"packages":["packages/*"],"catalog":{"core":"npm:real@^1"}}}"#,
    ),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"catalog:"}}"#,
    ),
    ("packages/real/package.json", r#"{"name":"real"}"#),
];

/// A Bun named catalog at the top level of the root manifest.
pub(crate) const BUN_TOP_LEVEL_CATALOGS: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"root","workspaces":["packages/*"],"catalogs":{"testing":{"core":"npm:real@^1"}}}"#,
    ),
    (
        "packages/app/package.json",
        r#"{"name":"app","devDependencies":{"core":"catalog:testing"}}"#,
    ),
    ("packages/real/package.json", r#"{"name":"real"}"#),
];

/// The same Bun repository with no catalog written anywhere.
pub(crate) const BUN_NO_CATALOG: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"root","workspaces":{"packages":["packages/*"]}}"#,
    ),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"catalog:"}}"#,
    ),
    ("packages/real/package.json", r#"{"name":"real"}"#),
];

/// npm and Yarn workspaces, a list and an object, each with a plain dependency.
pub(crate) const NPM_WORKSPACES: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"root","workspaces":["packages/*"]}"#,
    ),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"*"}}"#,
    ),
    ("packages/core/package.json", r#"{"name":"core"}"#),
];
pub(crate) const YARN_WORKSPACES: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"root","workspaces":{"packages":["packages/*"],"nohoist":["**/x"]}}"#,
    ),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"*"}}"#,
    ),
    ("packages/core/package.json", r#"{"name":"core"}"#),
];

/// The path and text of the sample monorepo's TSX file.
pub(crate) const APP_PATH: &str = "packages/app/src/App.tsx";
pub(crate) const APP_SOURCE: &str =
    "import { Button } from 'ui';\nexport const App = () => <Button />;\n";
/// The path and text of the sample monorepo's single-file component, whose script is TypeScript.
pub(crate) const CARD_PATH: &str = "packages/ui/src/Card.vue";
pub(crate) const CARD_SOURCE: &str =
    "<template><p/></template>\n<script lang=\"ts\">\nimport { ref } from 'vue';\n</script>\n";

/// A pnpm monorepo of two packages: the app depends on the ui package, and holds a TSX file; the ui package holds a component file.
pub(crate) const MONOREPO: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root","private":true}"#),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"ui":"workspace:*"}}"#,
    ),
    (APP_PATH, APP_SOURCE),
    ("packages/ui/package.json", r#"{"name":"ui"}"#),
    (CARD_PATH, CARD_SOURCE),
    ("pnpm-workspace.yaml", "packages:\n  - 'packages/*'\n"),
];

/// An unnamed root package depending on a named package.
pub(crate) const UNNAMED_ROOT: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"private":true,"dependencies":{"core":"*"}}"#,
    ),
    ("index.ts", "export {};"),
    ("core/package.json", r#"{"name":"core"}"#),
    ("core/index.ts", "export {};"),
];

/// A named root with an unnamed nested package depending on a named package.
pub(crate) const UNNAMED_NESTED: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    (
        "app/package.json",
        r#"{"private":true,"dependencies":{"core":"*"}}"#,
    ),
    ("app/index.ts", "export {};"),
    ("core/package.json", r#"{"name":"core"}"#),
];

/// A named root whose project reference points into an unnamed package: no name exists to call the edge.
pub(crate) const UNNAMED_REFERENCE: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    ("tsconfig.json", r#"{"references":[{"path":"app"}]}"#),
    ("app/package.json", r#"{"private":true}"#),
    ("app/tsconfig.json", "{}"),
];

/// Optional, required and metadata-free peer dependencies.
pub(crate) const PEERS: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"app","peerDependencies":{"core":"*","req":"*","bare":"*"},"peerDependenciesMeta":{"core":{"optional":true},"req":{"optional":false}}}"#,
    ),
    ("bare/package.json", r#"{"name":"bare"}"#),
    ("core/package.json", r#"{"name":"core"}"#),
    ("req/package.json", r#"{"name":"req"}"#),
];

/// An npm workspace whose root overrides what its members depend on: a plain key, a scoped key's `.`, a key that selects by version,
/// and Yarn's `**/` resolution; a member depends on a git repository at a commit-ish.
pub(crate) const NPM_OVERRIDES: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"root","workspaces":["packages/*"],"overrides":{"core":"2.0.0","@scope/lib":{".":"1.0.0","dep":"9.0.0"},"left@1":"3.0.0"},"resolutions":{"**/right":"4.0.0"}}"#,
    ),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"^1","@scope/lib":"^0.1","left":"^1","right":"^2","plain":"github:owner/plain#main","ranged":"github:owner/ranged#semver:^1"}}"#,
    ),
];

/// An npm workspace whose member writes overrides of its own, which npm does not apply: only the root's are.
pub(crate) const MEMBER_OVERRIDES: &[(&str, &str)] = &[
    (
        "package.json",
        r#"{"name":"root","workspaces":["packages/*"],"overrides":{"core":"2.0.0"}}"#,
    ),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"^1","own":"^1"},"overrides":{"own":"5.0.0","core":"9.0.0"}}"#,
    ),
];

/// A pnpm workspace file that overrides a package everywhere, and one only under a parent.
pub(crate) const PNPM_OVERRIDES: &[(&str, &str)] = &[
    ("package.json", r#"{"name":"root"}"#),
    (
        "packages/app/package.json",
        r#"{"name":"app","dependencies":{"core":"^1","deep":"^1"}}"#,
    ),
    (
        "pnpm-workspace.yaml",
        "packages:\n  - 'packages/*'\noverrides:\n  core: 2.0.0\n  'parent>deep': 1.0.0\n",
    ),
];
