//! `query validate`'s workload.

/// `query validate` of one query, run once per round, which must succeed.
pub(crate) fn workload() -> crate::measure::Measured {
    crate::measure::rounds(
        "query/validate",
        || (),
        |()| {
            let line = <ryo_cli::resources::query::validate::command::ValidateArguments as clap::Args>::augment_args(
                clap::Command::new("validate"),
            );
            let arguments = <ryo_cli::resources::query::validate::command::ValidateArguments as clap::FromArgMatches>::from_arg_matches(
                &line.get_matches_from(["validate", "--text", "let f = select * from \"function\"; emit f subject file key(file);"]),
            );
            assert!(arguments.is_ok(), "`query validate` parses with its text");
            let Ok(arguments) = arguments else { return };
            let outcome = ryo_cli::resources::query::validate::command::run(
                arguments,
                &crate::measure::quiet_edge("."),
            );
            assert_eq!(
                outcome,
                ryo_cli::outcome::CommandOutcome::Success,
                "`query validate` did not succeed"
            );
        },
    )
}
