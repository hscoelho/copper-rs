use cu29::clock::RobotClock;
use cu29::config::ComponentConfig;
use cu29::cutask::{CuSinkTask, CuTask, CuTaskLifecycle};
use cu29::cutask::{CuSrcTask, Freezable};

use cu29::cutask::CuMsg;
use cu29::{input_msg, output_msg};
use cu29_traits::CuResult;

/// A source task that generates an integer at each cycle.
pub struct IntegerSrcTask {
    pub value: i32,
}

impl Freezable for IntegerSrcTask {}

impl CuTaskLifecycle for IntegerSrcTask {
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self { value: 42 })
    }
}

impl<'cl> CuSrcTask<'cl> for IntegerSrcTask {
    type Output = output_msg!('cl, i32);

    fn process(&mut self, _clock: &RobotClock, output: Self::Output) -> CuResult<()> {
        self.value += 1;
        output.set_payload(self.value);
        Ok(())
    }
}

/// Example Source that produces a float at each cycle.
pub struct FloatSrcTask {
    pub value: f32,
}

impl Freezable for FloatSrcTask {}

impl CuTaskLifecycle for FloatSrcTask {
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self { value: 24.0 })
    }
}

impl<'cl> CuSrcTask<'cl> for FloatSrcTask {
    type Output = output_msg!('cl, f32);

    fn process(&mut self, _clock: &RobotClock, output: Self::Output) -> CuResult<()> {
        self.value += 1.0;
        output.set_payload(self.value);
        Ok(())
    }
}

/// Example Sink that receives an integer and a float from the 2 sources (IntegerSrcTask and FloatSrcTask).
pub struct MergingSinkTask {}

impl Freezable for MergingSinkTask {}

impl CuTaskLifecycle for MergingSinkTask {
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self {})
    }
}

impl<'cl> CuSinkTask<'cl> for MergingSinkTask {
    /// The input is an i32 from the IntegerSrcTask and a f32 from the FloatSrcTask.
    type Input = input_msg!('cl, i32, f32);

    fn process(&mut self, _clock: &RobotClock, input: Self::Input) -> CuResult<()> {
        let (i, f) = input;
        println!(
            "SinkTask1 received: {}, {}",
            i.payload().unwrap(),
            f.payload().unwrap()
        );
        Ok(())
    }
}

/// Example Task that merges the integer and float messages into a tuple message.
pub struct MergerTask {}

impl Freezable for MergerTask {}

impl CuTaskLifecycle for MergerTask {
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self {})
    }
}

impl<'cl> CuTask<'cl> for MergerTask {
    /// The input is an i32 from the IntegerSrcTask and a f32 from the FloatSrcTask.
    type Input = input_msg!('cl, i32, f32);

    /// The output is a tuple of i32 and f32.
    type Output = output_msg!('cl, (i32, f32));

    fn process(
        &mut self,
        _clock: &RobotClock,
        input: Self::Input,
        output: Self::Output,
    ) -> CuResult<()> {
        // Put the types explicitly here show the actual underlying type of Self::Input
        let (i, f): (&CuMsg<i32>, &CuMsg<f32>) = input;
        let (i, f) = (i.payload().unwrap(), f.payload().unwrap());
        output.set_payload((*i, *f)); // output is a &mut CuMsg<(i32, f32)>
        Ok(())
    }
}

/// Sink to close off the pipeline correctly fed from the merger task.
pub struct MergedSinkTask {}

impl Freezable for MergedSinkTask {}

impl CuTaskLifecycle for MergedSinkTask {
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self {})
    }
}

impl<'cl> CuSinkTask<'cl> for MergedSinkTask {
    type Input = input_msg!('cl, (i32, f32));

    fn process(&mut self, _clock: &RobotClock, input: Self::Input) -> CuResult<()> {
        println!("SinkTask2 received: {:?}", input.payload().unwrap());
        Ok(())
    }
}
