use cu29::prelude::*;

#[derive(Default)]
pub struct SrcTask {
    first_execution: bool,
}

impl Freezable for SrcTask {}

impl<'cl> CuSrcTask<'cl> for SrcTask {
    type Output = output_msg!('cl, String);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self {
            first_execution: true,
        })
    }

    fn process(&mut self, _clock: &RobotClock, output: Self::Output) -> CuResult<()> {
        if self.first_execution {
            // uncommenting this line solves the panic
            // output.clear_payload();
        } else {
            output.set_payload("SrcTask is running".into());
        }
        self.first_execution = false;

        Ok(())
    }
}

#[derive(Default)]
pub struct SinkTask {}

impl Freezable for SinkTask {}

impl<'cl> CuSinkTask<'cl> for SinkTask {
    type Input = input_msg!('cl, String);

    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self::default())
    }

    fn process(&mut self, _clock: &RobotClock, input: Self::Input) -> CuResult<()> {
        if let Some(msg) = input.payload() {
            println!("Sink received: {}", msg);
        }
        Ok(())
    }
}
