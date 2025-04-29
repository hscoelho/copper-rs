use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;

#[copper_runtime(config = "listener.ron")]
struct ListenerApplication {}

const SLAB_SIZE: Option<usize> = Some(100 * 1024 * 1024);

fn main() {
    let logger_path = "/tmp/zenoh_pong.copper";

    let copper_ctx = basic_copper_setup(&PathBuf::from(logger_path), SLAB_SIZE, false, None)
        .expect("Failed to setup logger.");
    let mut application = ListenerApplicationBuilder::new()
        .with_context(&copper_ctx)
        .build()
        .expect("Failed to create application.");
    application.run().expect("Failed to run application");
}
