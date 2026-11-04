//! Example
use swiftide::agents::Agent;

#[tokio::main]
async fn main() {
    // ANCHOR: build
    let agent = Agent::builder().build();
    // ANCHOR_END: build
}
