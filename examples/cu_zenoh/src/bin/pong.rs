use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;
use std::str;

#[copper_runtime(config = "pong.ron")]
struct PongApplication {}

const SLAB_SIZE: Option<usize> = Some(100 * 1024 * 1024);

pub struct PingHandlerTask {}
impl Freezable for PingHandlerTask {}

impl<'cl> CuTask<'cl> for PingHandlerTask {
    type Input = input_msg!('cl, Vec<u8>);
    type Output = output_msg!('cl, String);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self {})
    }

    fn process(
        &mut self,
        _clock: &RobotClock,
        input: Self::Input,
        output: Self::Output,
    ) -> CuResult<()> {
        if let Some(msg) = input.payload() {
            let s =
                str::from_utf8(msg).map_err(|_| CuError::from("Received payload is not utf8"))?;
            println!("Message received: {}", s);
            let out_msg = "Pong";
            println!("Sending message: {}", out_msg);
            output.set_payload(out_msg.into());
        } else {
            output.clear_payload();
        }

        Ok(())
    }
}

fn main() {
    let logger_path = "/tmp/zenoh_pong.copper";

    let copper_ctx = basic_copper_setup(&PathBuf::from(logger_path), SLAB_SIZE, false, None)
        .expect("Failed to setup logger.");
    let mut application = PongApplicationBuilder::new()
        .with_context(&copper_ctx)
        .build()
        .expect("Failed to create application.");
    application.run().expect("Failed to run application");
}
