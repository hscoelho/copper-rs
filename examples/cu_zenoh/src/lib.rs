use cu29::prelude::*;
use std::str;
use zenoh::bytes::ZBytes;
use zenoh::handlers::FifoChannelHandler;
use zenoh::pubsub::Publisher;
use zenoh::pubsub::Subscriber;
use zenoh::sample::Sample;
use zenoh::Session;
use zenoh::Wait;

pub type ZenohStringPublisherTask = ZenohPublisherTask<String>;

pub struct ZenohPublisherTask<P>
where
    P: CuMsgPayload + Into<ZBytes> + 'static,
{
    publisher: Publisher<'static>,
    // the session is stored because dropping the session closes the connection
    _session: Session,
    _marker: std::marker::PhantomData<P>,
}

impl<P> Freezable for ZenohPublisherTask<P> where P: CuMsgPayload + Into<ZBytes> + 'static {}

impl<'cl, P> CuSinkTask<'cl> for ZenohPublisherTask<P>
where
    P: CuMsgPayload + Into<ZBytes> + 'static,
{
    type Input = input_msg!('cl, P);
    fn new(config: Option<&ComponentConfig>) -> CuResult<Self> {
        let config = config.ok_or_else(|| CuError::from("You need a config."))?;
        let session = zenoh::open(zenoh::Config::default())
            .wait()
            .map_err(|_| CuError::from("Failed to open zenoh session"))?;
        let topic = config
            .get::<String>("topic")
            .ok_or_else(|| CuError::from("You need a topic"))?;
        let publisher = session
            .declare_publisher(topic)
            .wait()
            .map_err(|_| CuError::from("Failed to create zenoh publisher"))?;

        Ok(Self {
            publisher,
            _session: session,
            _marker: std::marker::PhantomData,
        })
    }

    fn process(&mut self, _clock: &RobotClock, input: Self::Input) -> CuResult<()> {
        if let Some(payload) = input.payload() {
            self.publisher
                .put(payload.clone())
                .wait()
                .map_err(|_| CuError::from("Failed to publish value"))?;
        }
        Ok(())
    }
}

pub struct ZenohSubscriberTask {
    subscriber: Subscriber<FifoChannelHandler<Sample>>,
    // the session is stored because dropping the session closes the connection
    _session: Session,
}

impl Freezable for ZenohSubscriberTask {}

impl<'cl> CuSrcTask<'cl> for ZenohSubscriberTask {
    // not sure about the payload being a vector
    type Output = output_msg!('cl, Vec<u8>);
    fn new(config: Option<&ComponentConfig>) -> CuResult<Self> {
        let config = config.ok_or_else(|| CuError::from("You need a config."))?;
        let session = zenoh::open(zenoh::Config::default())
            .wait()
            .map_err(|_| CuError::from("Failed to open zenoh session"))?;
        let topic = config
            .get::<String>("topic")
            .ok_or_else(|| CuError::from("You need a topic"))?;
        let subscriber = session
            .declare_subscriber(topic)
            .wait()
            .map_err(|_| CuError::from("Failed to declare zenoh subscriber."))?;
        Ok(Self {
            _session: session,
            subscriber,
        })
    }

    fn process(&mut self, _clock: &RobotClock, output: Self::Output) -> CuResult<()> {
        match self.subscriber.try_recv() {
            Ok(Some(sample)) => {
                let bytes = sample.payload().to_bytes();
                output.set_payload(Vec::from(bytes.clone()));
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

pub struct PrintTask {}
impl Freezable for PrintTask {}

impl<'cl> CuSinkTask<'cl> for PrintTask {
    type Input = input_msg!('cl, Vec<u8>);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self {})
    }

    fn process(&mut self, _clock: &RobotClock, input: Self::Input) -> CuResult<()> {
        if let Some(msg) = input.payload() {
            let s =
                str::from_utf8(msg).map_err(|_| CuError::from("Received payload is not utf8"))?;
            println!("Message received: {}", s);
        }
        Ok(())
    }
}
