use cucumber::{World as _, then, when};
use std::process::Output;

mod support;

#[derive(Debug, cucumber::World)]
struct NativeWorld {
    consumer: support::NativeConsumer,
    output: Option<Output>,
}

impl Default for NativeWorld {
    fn default() -> Self {
        Self {
            consumer: support::NativeConsumer::new("bdd"),
            output: None,
        }
    }
}

#[when("a generic implementation is restricted to nominal owners")]
fn generic_nominal(world: &mut NativeWorld) {
    world.output = Some(
        world
            .consumer
            .check(include_str!("fixtures/generic_receiver.rs")),
    );
}

#[when("the bound owner also admits a primitive")]
fn generic_primitive(world: &mut NativeWorld) {
    world.output = Some(
        world
            .consumer
            .check(include_str!("fixtures/generic_receiver_mixed.rs")),
    );
}

#[when("the implementation supplies a nominal type")]
fn nominal(world: &mut NativeWorld) {
    world.output = Some(world.consumer.check(include_str!("fixtures/nominal.rs")));
}

#[when("an unrelated Rust trait exposes a primitive")]
fn unselected(world: &mut NativeWorld) {
    world.output = Some(world.consumer.check(include_str!("fixtures/unselected.rs")));
}

#[when("the implementation supplies a foreign primitive alias")]
fn primitive(world: &mut NativeWorld) {
    world.output = Some(
        world
            .consumer
            .check(include_str!("fixtures/associated_binding.rs")),
    );
}

#[when("the implementation leaves its boundary type unresolved")]
fn unresolved(world: &mut NativeWorld) {
    world.output = Some(
        world
            .consumer
            .check(include_str!("fixtures/generic_boundary.rs")),
    );
}

#[then("native checking accepts the implementation")]
fn accepted(world: &mut NativeWorld) {
    support::assert_success(world.output.take().expect("completed native check"));
}

#[then("native checking rejects the representation")]
fn disallowed(world: &mut NativeWorld) {
    rejected(world, "nominal_boundary_type");
}

#[then("native checking reports an unproven boundary")]
fn unproven(world: &mut NativeWorld) {
    rejected(world, "unproven_boundary_type");
}

fn rejected(world: &mut NativeWorld, code: &str) {
    let output = world.output.take().expect("completed native check");
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout).lines().any(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .is_ok_and(|message| message["message"]["code"]["code"] == code)
        }),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    NativeWorld::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/bdd.feature"))
        .await;
}
