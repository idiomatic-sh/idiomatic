//! The product README's Usage, each command run and its output compared.

/// A new project's directory: the git repository Asymptote reads and an `asymptote.toml` with the two tools the Usage
/// names.
#[expect(
    clippy::expect_used,
    reason = "test support: a scene that cannot be set up is a broken test, and the message names which"
)]
fn project(root: &std::path::Path) {
    for directory in [".git/objects", ".git/refs/heads"] {
        std::fs::create_dir_all(root.join(directory)).expect("the project's repository is made");
    }
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n")
        .expect("its branch is written");
    std::fs::write(
        root.join("asymptote.toml"),
        "[[tools]]\nid = \"lint\"\ngroup = \"lint\"\nrun = [\"true\"]\n\n[[tools]]\nid = \"check\"\ngroup = \"check\"\nrun = [\"true\"]\n\n[hooks]\npre-commit = [\"lint\"]\n",
    )
    .expect("the manifest is written");
}

/// The README's commands run as a new project's would: one after another in a directory of their own, holding the git
/// repository Asymptote reads and an `asymptote.toml` with the two tools the Usage names. trycmd runs a Markdown file's
/// blocks in a sandbox when an `.out` directory sits beside it, starting from the `.in` directory beside it, so the
/// README is copied beside both.
#[test]
fn readme() {
    let copy = tempfile::tempdir().expect("a directory for the README's copy");
    let readme = copy.path().join("README.md");
    let _: u64 = std::fs::copy("../README.md", &readme).expect("the README is copied");
    std::fs::create_dir_all(copy.path().join("README.out")).expect("the sandbox marker is made");
    project(&copy.path().join("README.in"));
    let cases = trycmd::TestCases::new();
    // `case` hands the cases back for chaining; they run when `cases` is dropped, at the end of the test.
    let _: &trycmd::TestCases = cases.case(&readme);
    drop(cases);
}

/// The README's `sh` blocks as they are written, comments and all: each command runs in a new project, one after
/// another, and answers as a command that parsed does, not as a usage error.
#[test]
fn readme_sh_blocks() {
    let readme = std::fs::read_to_string("../README.md").expect("the README is read");
    let directory = tempfile::tempdir().expect("a directory for the project");
    project(directory.path());
    let mut inside = false;
    let mut ran = 0_usize;
    for line in readme.lines() {
        if line.starts_with("```") {
            inside = line == "```sh";
            continue;
        }
        let command = line.split(" #").next().unwrap_or_default().trim();
        if !inside || command.is_empty() {
            continue;
        }
        let words = shlex::split(command).expect("the command's quotes close");
        let (program, arguments) = words.split_first().expect("a command has words");
        assert_eq!(program, "asymptote", "`{command}` runs asymptote");
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_asymptote"))
            .args(arguments)
            .current_dir(directory.path())
            .output()
            .expect("the command runs");
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            output.status.code().is_some() && !said.is_empty(),
            "`{command}` ended and said something"
        );
        for usage in ["unexpected argument", "unrecognized subcommand"] {
            assert!(!said.contains(usage), "`{command}` is a command: {said}");
        }
        ran += 1;
    }
    assert!(ran > 0, "the README has an `sh` block to run");
}
