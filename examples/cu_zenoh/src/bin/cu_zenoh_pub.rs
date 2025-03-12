use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;

#[copper_runtime(config = "publisher.ron")]
struct PublisherApplication {}

pub struct ZenohPublisherTask {}
impl Freezable for ZenohPublisherTask {}

impl<'cl> CuSinkTask<'cl> for ZenohPublisherTask {
    type Input = input_msg!('cl, i32);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: Self::Input) -> CuResult<()> {
        println!("Input received: {}", input.payload().unwrap());
        Ok(())
    }
}

pub struct ValueSupplierTask {
    pub value: i32,
}

impl Freezable for ValueSupplierTask {}

impl<'cl> CuSrcTask<'cl> for ValueSupplierTask {
    type Output = output_msg!('cl, i32);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self { value: 42 })
    }

    fn process(&mut self, _clock: &RobotClock, output: Self::Output) -> CuResult<()> {
        self.value += 1;
        output.set_payload(self.value);
        Ok(())
    }
}

const SLAB_SIZE: Option<usize> = Some(100 * 1024 * 1024);

fn main() {
    let logger_path = "/tmp/downstream.copper";

    let copper_ctx = basic_copper_setup(&PathBuf::from(logger_path), SLAB_SIZE, false, None)
        .expect("Failed to setup logger.");
    let mut application = PublisherApplicationBuilder::new()
        .with_context(&copper_ctx)
        .build()
        .expect("Failed to create application.");

    let clock = copper_ctx.clock;
    debug!("Running... starting clock: {}.", clock.now());

    application
        .run_one_iteration()
        .expect("Failed to run application");
    debug!("End of program: {}.", clock.now());
}
