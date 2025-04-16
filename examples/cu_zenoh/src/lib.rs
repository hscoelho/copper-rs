use cu29::prelude::*;
use zenoh::handlers::FifoChannelHandler;
use zenoh::pubsub::Subscriber;
use zenoh::sample::Sample;
use zenoh::Session;
use zenoh::Wait;

// TODO: Remove all the unwraps

pub struct ZenohPublisherTask {
    session: Session,
    // TODO: Store a publisher instead of the topic
    topic: String,
}
impl Freezable for ZenohPublisherTask {}

impl<'cl> CuSinkTask<'cl> for ZenohPublisherTask {
    type Input = input_msg!('cl, String);
    fn new(config: Option<&ComponentConfig>) -> CuResult<Self> {
        let config = config.ok_or_else(|| CuError::from("You need a config."))?;
        let session = zenoh::open(zenoh::Config::default()).wait().unwrap();
        let topic = config
            .get::<String>("topic")
            .ok_or_else(|| CuError::from("You need a topic"))?;

        Ok(Self { session, topic })
    }

    fn process(&mut self, _clock: &RobotClock, input: Self::Input) -> CuResult<()> {
        if let Some(val) = input.payload() {
            self.session.put(self.topic.clone(), val).wait().unwrap();
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
    fn new(config: Option<&ComponentConfig>) -> CuResult<Self> {
        let config = config.ok_or_else(|| CuError::from("You need a config."))?;
        let session = zenoh::open(zenoh::Config::default()).wait().unwrap();
        let topic = config
            .get::<String>("topic")
            .ok_or_else(|| CuError::from("You need a topic"))?;
        let subscriber = session.declare_subscriber(topic).wait().unwrap();
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
