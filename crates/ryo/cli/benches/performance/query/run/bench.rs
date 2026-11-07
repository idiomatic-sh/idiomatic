//! `query run`'s workload.

/// `query run` of a query over the sample, run once per round, which must succeed.
pub(crate) fn workload() -> crate::measure::Measured {
    crate::measure::rounds(
        "query/run",
        || tempfile::tempdir().expect("a directory to run over"),
        |root| {
            let line =
                <ryo_cli::resources::query::run::command::RunArguments as clap::Args>::augment_args(
                    clap::Command::new("run"),
                );
            let arguments = <ryo_cli::resources::query::run::command::RunArguments as clap::FromArgMatches>::from_arg_matches(
                &line.get_matches_from([
                    "run",
                    "--text",
                    "let f = select * from \"function\"; emit f subject file key(file);",
                ]),
            );
            assert!(arguments.is_ok(), "`query run` parses its text");
            let Ok(arguments) = arguments else { return };
            let outcome = ryo_cli::resources::query::run::command::run(
                arguments,
                &crate::measure::quiet_edge(root.path().to_str().expect("a utf-8 path")),
            );
            assert_eq!(
                outcome,
                ryo_cli::outcome::CommandOutcome::Success,
                "`query run` did not succeed"
            );
        },
    )
}
