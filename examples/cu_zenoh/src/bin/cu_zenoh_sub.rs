use cu29::prelude::*;
use cu29_helpers::basic_copper_setup;
use std::path::PathBuf;
use zenoh::handlers::FifoChannelHandler;
use zenoh::pubsub::Subscriber;
use zenoh::sample;
use zenoh::sample::Sample;
use zenoh::Session;
use zenoh::Wait;

pub struct PrintTask {}
impl Freezable for PrintTask {}

impl<'cl> CuSinkTask<'cl> for PrintTask {
    type Input = input_msg!('cl, String);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: Self::Input) -> CuResult<()> {
        if let Some(msg) = input.payload() {
            println!("Input received: {}", msg);
        }
        Ok(())
    }
}

pub struct ZenohSubscriberTask {
    // the session is stored to make sure it's not dropped
    // since dropping the session closes the connection
    _session: Session,
    subscriber: Subscriber<FifoChannelHandler<Sample>>,
}

impl Freezable for ZenohSubscriberTask {}

impl<'cl> CuSrcTask<'cl> for ZenohSubscriberTask {
    type Output = output_msg!('cl, String);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        let session = zenoh::open(zenoh::Config::default()).wait().unwrap();
        let subscriber = session.declare_subscriber("topic").wait().unwrap();
        Ok(Self {
            _session: session,
            subscriber,
        })
    }

    fn process(&mut self, _clock: &RobotClock, output: Self::Output) -> CuResult<()> {
        match self.subscriber.try_recv() {
            Ok(Some(sample)) => {
                let s = sample.payload().try_to_string().unwrap().into_owned();
                output.set_payload(s);
                Ok(())
            }
            Ok(None) => {
                output.clear_payload();
                Ok(())
            }
            Err(e) => {
                let s = format!("Error receiving message: {:?}", e);
                Err(CuError::from(s))
            }
        }
    }
}

#[copper_runtime(config = "subscriber.ron")]
struct SubscriberApplication {}

const SLAB_SIZE: Option<usize> = Some(100 * 1024 * 1024);

fn main() {
    let logger_path = "/tmp/zenoh_sub.copper";

    let copper_ctx = basic_copper_setup(&PathBuf::from(logger_path), SLAB_SIZE, false, None)
        .expect("Failed to setup logger.");
    let mut application = SubscriberApplicationBuilder::new()
        .with_context(&copper_ctx)
        .build()
        .expect("Failed to create application.");
    application.run().expect("Failed to run application");
}
