//! The public flag reading.
use cucumber::then;
use ryo_benchmark::inbound::use_cases::select_cases::{
    interface::SelectCasesUseCase as _, types::CaseArgument, use_case::SelectCasesUseCaseImpl,
};
use ryo_benchmark_contracts::domain::requests::types::ShardPartition;

fn select(flags: &[&str]) -> Result<ShardPartition, String> {
    let arguments = flags
        .iter()
        .map(|flag| CaseArgument((*flag).to_owned()))
        .collect::<Vec<_>>();
    SelectCasesUseCaseImpl
        .select_cases(&arguments)
        .map(|run| run.shard)
        .map_err(|error| error.to_string())
}

#[then("select_cases reads the shard a flag names")]
fn reads(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(matches!(
        select(&["--shard=1/3"]),
        Ok(ShardPartition::Shard(_))
    ));
}

#[then("select_cases refuses a repeated shard flag")]
fn refuses(world: &mut crate::Behavior) {
    let _: &mut crate::Behavior = world;
    assert!(select(&["--shard=0/2", "--shard=1/2"]).is_err());
}
