pub mod tasks;

use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

const PREALLOCATED_STORAGE_SIZE: Option<usize> = Some(1024 * 1024 * 100);

#[copper_runtime(config = "copperconfig.ron")]
struct CuZenohApplication {}

fn main() {
    let logger_path = "cu-zenoh.copper";
    let copper_ctx =
        basic_copper_setup(&PathBuf::from(logger_path), PREALLOCATED_STORAGE_SIZE, true, None).expect("Failed to setup logger.");
    debug!("Logger created at {}.", logger_path);
    debug!("Creating application... ");
    let mut application = CuZenohApplicationBuilder::new()
            .with_context(&copper_ctx)
            .build()
            .expect("Failed to create application.");
    let clock = copper_ctx.clock.clone();
    debug!("Running... starting clock: {}.", clock.now());

    application.run().expect("Failed to run application.");
    debug!("End of program: {}.", clock.now());
    sleep(Duration::from_secs(1));
}
