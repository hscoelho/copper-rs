use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;

#[copper_runtime(config = "ping.ron")]
struct PingApplication {}

pub struct PingTask {
    pinged: bool,
}
impl Freezable for PingTask {}

impl<'cl> CuSrcTask<'cl> for PingTask {
    type Output = output_msg!('cl, String);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self { pinged: false })
    }

    fn process(&mut self, _clock: &RobotClock, output: Self::Output) -> CuResult<()> {
        if !self.pinged {
            println!("Sending message: Ping");
            output.set_payload("Ping".into());
            self.pinged = true;
        } else {
            output.clear_payload();
        }
        Ok(())
    }
}
const SLAB_SIZE: Option<usize> = Some(100 * 1024 * 1024);

fn main() {
    let logger_path = "/tmp/zenoh_pong.copper";

    let copper_ctx = basic_copper_setup(&PathBuf::from(logger_path), SLAB_SIZE, false, None)
        .expect("Failed to setup logger.");
    let mut application = PingApplicationBuilder::new()
        .with_context(&copper_ctx)
        .build()
        .expect("Failed to create application.");
    application.run().expect("Failed to run application");
}
