//! Entry point of the BDD suite.
//!
//! Scenarios run one at a time: they share a single test database and each one
//! starts by emptying it, which only holds if nothing else is writing to it
//! meanwhile.

mod support;

use cucumber::World as _;

use crate::support::world::World;

#[tokio::main]
async fn main() {
    World::cucumber()
        .max_concurrent_scenarios(1)
        .fail_on_skipped()
        .run_and_exit("tests/features")
        .await;
}
