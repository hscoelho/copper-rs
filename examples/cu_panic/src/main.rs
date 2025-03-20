pub mod tasks;

use crate::tasks::*;
use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

#[copper_runtime(config = "copperconfig.ron")]
struct App {}

const SLAB_SIZE: Option<usize> = Some(1024 * 1024);

fn main() {
    let logger_path = "/tmp/panic_logger.copper";
    let copper_ctx = basic_copper_setup(&PathBuf::from(logger_path), SLAB_SIZE, true, None)
        .expect("Failed to setup logger.");
    debug!("Logger created at {}.", path = logger_path);
    debug!("Creating application... ");
    let mut application = AppBuilder::new()
        .with_context(&copper_ctx)
        .build()
        .expect("Failed to create runtime.");

    let clock = copper_ctx.clock;
    debug!("Running... starting clock: {}.", clock.now());

    application.run().expect("Failed to run application.");
    debug!("End of program: {}.", clock.now());
    sleep(Duration::from_secs(1));
}
