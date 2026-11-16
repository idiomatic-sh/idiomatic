//! Ruff's growth plan: how the rules that resolve a name grow with the file, in every shape.
//!
//! The shapes are generated from one table: each operation (bind, rebind, import, handler, loop and with targets, walrus,
//! annotation, def, class, read, delete, `global` and `nonlocal` declarations, defaults, lambdas and comprehensions, many
//! functions reading a rebound module name, many distinct names, a read run of more than 16,384 reads) repeated in every scope
//! kind it makes sense in (module, function, nested function, class body, method). Each of F401, F841, F811 and F821 runs
//! every program at 500 and 1,000 repetitions; the shared growth service lints them and holds each shape to the one CPU limit
//! and to Ruff's floor and memory bound. A rule that joins every read to every binding, pairs every effect with every other,
//! or tests every handler against every read grows 3.5 to 8 times per doubling and holds gigabytes.
use super::constants;
use super::locations::directory;
use ryo_benchmark::domain::growth::constants::{CPU_FLOOR, MEMORY_BOUND};
use ryo_benchmark::domain::growth::types::{
    GrownProgram, GrowthModule, GrowthPlan, GrowthSize, RuleModules, ShapePrograms, SourceFileName,
};
use ryo_benchmark::outbound::suite::{interface::SuitePort, types::ArtifactPath};
use ryo_benchmark_contracts::domain::measurements::types::GrowthShape;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::{
    CohortSelection, ContextSeal, InputDeclaration, ParserSelection, ProviderSelection,
    RuleContextPreparation, SearchRoots, WorkerRequest, WorkerState,
};
use std::collections::BTreeMap;

/// The rules whose cost is grown over every shape.
const RULES: [&str; 4] = ["F401", "F841", "F811", "F821"];
/// The repetitions of each shape at the half and the whole size.
const HALF: usize = 500;
/// The repetitions of each shape at the whole size.
const WHOLE: usize = 1_000;
/// The reads after the repetitions of the long read run: one more than the engine's run of 16,384.
const LONG_READ_RUN: usize = 16_384;
/// The indentation of one block level.
const INDENT: &str = "    ";
/// Where a repetition's index is written in an operation's text.
const INDEX: &str = "{i}";

/// Where a shape's repetitions sit.
#[derive(Clone, Copy)]
enum Scope {
    /// At module level.
    Module,
    /// In a function body.
    Function,
    /// In a function nested in a function.
    Nested,
    /// In a class body.
    Class,
    /// In a method.
    Method,
}

impl Scope {
    /// Every scope kind.
    const ALL: [Self; 5] = [
        Self::Module,
        Self::Function,
        Self::Nested,
        Self::Class,
        Self::Method,
    ];

    /// The scope as a shape names it.
    const fn label(self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Function => "function",
            Self::Nested => "nested",
            Self::Class => "class",
            Self::Method => "method",
        }
    }
}

/// One operation: the lines before the repetitions and the lines repeated; `{i}` is the repetition's index.
struct Operation {
    /// The operation as a shape names it.
    name: &'static str,
    /// The lines before the repetitions.
    prologue: &'static str,
    /// The lines repeated.
    repeat: &'static str,
    /// Only meaningful at module level (it defines the functions or defaults it reads there).
    module_only: bool,
    /// Needs an enclosing function binding (a `nonlocal` declaration).
    nested_only: bool,
}

/// An operation meaningful in any scope its name allows.
const fn op(name: &'static str, prologue: &'static str, repeat: &'static str) -> Operation {
    Operation {
        name,
        prologue,
        repeat,
        module_only: false,
        nested_only: false,
    }
}

/// The operations whose code path depends on the scope kind; the others run at module level only.
const SCOPE_SENSITIVE: &[&str] = &[
    "assign-read",
    "import-read",
    "handler-read",
    "annotation-read",
    "assign-del",
    "global-assign",
    "global-read",
    "global-del",
    "nonlocal-assign",
    "distinct-assign",
];

/// Every operation grown.
const OPERATIONS: &[Operation] = &[
    op("assign", "", "v = 1\n"),
    op("assign-read", "", "v = 1\nprint(v)\n"),
    op("import", "", "import v\n"),
    op("import-read", "", "import v\nprint(v)\n"),
    op(
        "handler",
        "",
        "try:\n    pass\nexcept Exception as v:\n    pass\n",
    ),
    op(
        "handler-read",
        "",
        "try:\n    pass\nexcept Exception as v:\n    print(v)\n",
    ),
    op("for-target", "", "for v in y:\n    print(v)\n"),
    op("with-target", "", "with y as v:\n    print(v)\n"),
    op("walrus", "", "(v := 1)\nprint(v)\n"),
    op("annotation", "", "v: int\n"),
    op("annotation-read", "", "v: int\nprint(v)\n"),
    op("def", "", "def v():\n    pass\n"),
    op("class", "", "class v:\n    pass\n"),
    op("read", "v = 1\n", "print(v)\n"),
    op("assign-del", "", "v = 1\ndel v\n"),
    op("del", "v = 1\n", "del v\n"),
    op("global-declaration", "", "global v\n"),
    op("global-assign", "", "global v\nv = 1\n"),
    op("global-read", "", "global v\nprint(v)\n"),
    op("global-del", "", "global v\ndel v\n"),
    Operation {
        nested_only: true,
        ..op("nonlocal-assign", "", "nonlocal v\nv = 1\n")
    },
    op("distinct-import", "", "import v{i}\nprint(v{i})\n"),
    op("distinct-assign", "", "v{i} = 1\nprint(v{i})\n"),
    op("distinct-def", "", "def v{i}():\n    pass\n"),
    op("distinct-class", "", "class v{i}:\n    x = 1\n"),
    op(
        "distinct-handler",
        "",
        "try:\n    pass\nexcept Exception as v{i}:\n    print(v{i})\n",
    ),
    Operation {
        module_only: true,
        ..op("default-argument", "v = 1\n", "def g{i}(a=v):\n    pass\n")
    },
    Operation {
        module_only: true,
        ..op(
            "rebind-read-by-functions",
            "",
            "v = 1\ndef g{i}():\n    return v\n",
        )
    },
    Operation {
        module_only: true,
        ..op("lambda-read", "v = 1\n", "w{i} = lambda: v\n")
    },
    Operation {
        module_only: true,
        ..op("comprehension-read", "v = 1\n", "w{i} = [v for _ in y]\n")
    },
    op("method-read", "v = 1\n", "def m{i}(self):\n    return v\n"),
    op(
        "global-skipped-read",
        "global v\n",
        "v = 1\ndef g{i}():\n    return v\n",
    ),
    op("long-read-run", "", "v = 1\n"),
];

/// Whether the operation makes sense in the scope.
fn grown_in(scope: Scope, operation: &Operation) -> bool {
    let module = matches!(scope, Scope::Module);
    (module || !operation.module_only)
        && (module || SCOPE_SENSITIVE.contains(&operation.name))
        && (!operation.nested_only || matches!(scope, Scope::Nested))
}

/// The operation repeated `count` times in the scope.
fn program(scope: Scope, operation: &Operation, count: usize) -> GrownProgram {
    let repeated = |depth: usize| -> String {
        let pad = INDENT.repeat(depth);
        let line =
            |text: &str| -> String { text.lines().map(|line| format!("{pad}{line}\n")).collect() };
        let mut text = line(operation.prologue);
        for index in 0..count {
            text.push_str(&line(&operation.repeat.replace(INDEX, &index.to_string())));
        }
        if operation.name == "long-read-run" {
            text.push_str(&line(&"v\n".repeat(LONG_READ_RUN + count)));
        }
        text
    };
    GrownProgram(match scope {
        Scope::Module => repeated(0),
        Scope::Function => format!("def f():\n{}", repeated(1)),
        Scope::Nested => {
            let outer = if operation.nested_only {
                "    v = 1\n"
            } else {
                ""
            };
            format!("def f():\n{outer}    def g():\n{}", repeated(2))
        }
        Scope::Class => format!("class C:\n{}", repeated(1)),
        Scope::Method => format!("class C:\n    def m(self):\n{}", repeated(2)),
    })
}

/// Ruff's plan: every rule over every generated shape, each with the modules its own profile registers, linted with the
/// bench's parser configuration.
///
/// # Errors
/// A rule identity the SDK refuses, or a rule profile the suite cannot read.
pub(super) fn plan(suite: &dyn SuitePort, parser: &ParserSelection) -> Result<GrowthPlan, Error> {
    let bench = directory();
    let mut shapes = BTreeMap::new();
    let mut applicability = BTreeMap::new();
    for rule in RULES {
        let id = ryo::linter::RuleId::try_from(rule)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        let profile = suite.profile(ArtifactPath(
            &bench.join(constants::RULE_PROFILES_DIRECTORY).join(rule),
        ))?;
        applicability.extend(profile.rule_applicability);
        let modules = RuleModules(
            profile
                .query_modules
                .into_iter()
                .map(|(name, path)| (GrowthModule(name.0), path))
                .collect(),
        );
        for scope in Scope::ALL {
            for operation in OPERATIONS.iter().filter(|each| grown_in(scope, each)) {
                let _previous = shapes.insert(
                    GrowthShape(format!("{rule}/{}/{}", scope.label(), operation.name)),
                    ShapePrograms {
                        rule: id.clone(),
                        modules: modules.clone(),
                        half: program(scope, operation, HALF),
                        whole: program(scope, operation, WHOLE),
                        size: GrowthSize(WHOLE),
                    },
                );
            }
        }
    }
    Ok(GrowthPlan {
        floor: CPU_FLOOR,
        memory: MEMORY_BOUND,
        file: SourceFileName(constants::GROWTH_SOURCE.into()),
        request: WorkerRequest {
            performance: Default::default(),
            selected_rules: CohortSelection::Explicit(Vec::new()),
            rule_observations: InputDeclaration::Undeclared,
            rule_context: RuleContextPreparation::Unconfigured,
            project_search_roots: SearchRoots::Undeclared,
            given_project_inventory: Default::default(),
            project_context_mode: Default::default(),
            context_manifest: ContextSeal::Unsealed,
            rules: bench.join(constants::RULES_DIRECTORY),
            root: bench,
            query_modules: BTreeMap::new(),
            rule_applicability: applicability,
            parser_configuration: parser.clone(),
            source_language: ProviderSelection::Unspecified,
            files: Vec::new(),
            state: WorkerState::Fresh,
        },
        shapes,
    })
}
