use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;
use zenoh::Session;
use zenoh::Wait;

#[copper_runtime(config = "publisher.ron")]
struct PublisherApplication {}

pub struct ZenohPublisherTask {
    session: Session,
}
impl Freezable for ZenohPublisherTask {}

impl<'cl> CuSinkTask<'cl> for ZenohPublisherTask {
    type Input = input_msg!('cl, u8);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        let session = zenoh::open(zenoh::Config::default()).wait().unwrap();
        Ok(Self { session })
    }

    fn process(&mut self, _clock: &RobotClock, input: Self::Input) -> CuResult<()> {
        let val = input.payload().unwrap();
        println!("Input received: {}", val);

        self.session.put("topic", [*val]).wait().unwrap();
        println!("Published zenoh message: {}", val);
        Ok(())
    }
}

impl Drop for ZenohPublisherTask {
    fn drop(&mut self) {
        self.session.close().wait().unwrap();
    }
}

pub struct ValueSupplierTask {
    pub value: u8,
}

impl Freezable for ValueSupplierTask {}

impl<'cl> CuSrcTask<'cl> for ValueSupplierTask {
    type Output = output_msg!('cl, u8);
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
