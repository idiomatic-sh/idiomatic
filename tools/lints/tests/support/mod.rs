use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

#[derive(Debug)]
pub struct NativeConsumer {
    pub root: tempfile::TempDir,
}

impl NativeConsumer {
    /// A consumer workspace whose builds go to `target/policy-consumer/<cache>`: each test
    /// binary names its own, so concurrently running binaries never share a build.
    pub fn new(cache: &str) -> Self {
        let root = tempfile::tempdir().expect("independent native consumer");
        let lint = Path::new(env!("CARGO_MANIFEST_DIR"));
        let manifest = format!(
            "[workspace]\nmembers=['foreign','contract','consumer']\nresolver='3'\n\
         [workspace.metadata.dylint]\nlibraries=[{{path={:?}}}]\n",
            lint.to_str().expect("lint package path")
        );
        write(root.path(), "Cargo.toml", &manifest);
        write(
            root.path(),
            ".cargo/config.toml",
            &format!(
                "[build]\njobs=6\ntarget-dir={:?}\n",
                lint.join("target/policy-consumer")
                    .join(cache)
                    .to_str()
                    .expect("cache path")
            ),
        );
        write(
            root.path(),
            "rust-toolchain.toml",
            include_str!("../../rust-toolchain.toml"),
        );
        for package in ["foreign", "contract", "consumer"] {
            write(
                root.path(),
                &format!("{package}/Cargo.toml"),
                &format!(
                    "[package]\nname='{package}'\nversion='0.1.0'\nedition='2024'\n\
             [lints.rust.unexpected_cfgs]\nlevel='warn'\ncheck-cfg=['cfg(dylint_lib, values(\"idiomatic_lints\"))']\n{}",
                    if package == "consumer" {
                        "[dependencies]\nforeign={path='../foreign'}\ncontract={path='../contract'}\n"
                    } else {
                        ""
                    }
                ),
            );
        }
        write(
            root.path(),
            "foreign/src/lib.rs",
            include_str!("../fixtures/foreign.rs"),
        );
        write(
            root.path(),
            "contract/src/lib.rs",
            include_str!("../fixtures/contract.rs"),
        );
        write(
            root.path(),
            "consumer/src/lib.rs",
            include_str!("../fixtures/nominal.rs"),
        );
        assert_success(
            cargo(root.path())
                .args(["generate-lockfile", "--offline"])
                .output()
                .expect("lockfile producer"),
        );

        Self { root }
    }

    pub fn check(&self, source: &str) -> Output {
        write(self.root.path(), "consumer/src/lib.rs", source);
        assert_success(
            cargo(self.root.path())
                .args([
                    "check",
                    "--workspace",
                    "--offline",
                    "--locked",
                    "--jobs",
                    "2",
                ])
                .output()
                .expect("compiler baseline"),
        );
        self.run()
    }

    pub fn run(&self) -> Output {
        self.run_targets(false)
    }

    pub fn check_tests(&self, source: &str) -> Output {
        write(self.root.path(), "consumer/src/lib.rs", source);
        assert_success(
            cargo(self.root.path())
                .args([
                    "check",
                    "--workspace",
                    "--all-targets",
                    "--offline",
                    "--locked",
                    "--jobs",
                    "2",
                ])
                .output()
                .expect("test-target compiler baseline"),
        );
        self.run_targets(true)
    }

    fn run_targets(&self, tests: bool) -> Output {
        let mut command = cargo(self.root.path());
        command.args([
            "dylint",
            "--lib",
            "idiomatic_lints",
            "--workspace",
            "--",
            "--offline",
            "--locked",
            "--jobs",
            "2",
            "--message-format=json",
        ]);
        if tests {
            command.arg("--all-targets");
        }
        command.output().expect("native Dylint")
    }
}

/// `cargo` run in the consumer at `root`, building into the target its `.cargo/config.toml` names: a target the caller
/// names in the environment (`CARGO_TARGET_DIR`, as a shared target does) is never inherited, so consumers built at once
/// never share a build.
pub fn cargo(root: &Path) -> Command {
    let mut command = Command::new("cargo");
    command
        .current_dir(root)
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CARGO_BUILD_TARGET_DIR");
    command
}

pub fn write(root: &Path, relative: &str, source: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture parent")).expect("fixture directories");
    fs::write(path, source).expect("independent fixture source");
}

pub fn assert_success(output: Output) {
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
