use cu29::prelude::*;
use std::{fmt::Display, marker::PhantomData};

#[derive(Default)]
pub struct StrSrcTask {
    number_of_executions: i64,
}

impl Freezable for StrSrcTask {}

impl<'cl> CuSrcTask<'cl> for StrSrcTask {
    type Output = output_msg!('cl, String);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self {
            number_of_executions: 0,
        })
    }

    fn process(&mut self, _clock: &RobotClock, output: Self::Output) -> CuResult<()> {
        if self.number_of_executions == 0 {
            // output.clear_payload();
        } else if self.number_of_executions == 1 {
            output.set_payload("SrcTask ran one time".into());
        } else {
            output.clear_payload();
        }
        self.number_of_executions += 1;

        Ok(())
    }
}

#[derive(Default)]
pub struct IntSrcTask {
    number_of_executions: i64,
}

impl Freezable for IntSrcTask {}

impl<'cl> CuSrcTask<'cl> for IntSrcTask {
    type Output = output_msg!('cl, i64);
    fn new(_config: Option<&ComponentConfig>) -> CuResult<Self> {
        Ok(Self {
            number_of_executions: 0,
        })
    }

    fn process(&mut self, _clock: &RobotClock, output: Self::Output) -> CuResult<()> {
        if self.number_of_executions == 0 {
            // output.clear_payload();
        } else if self.number_of_executions == 1 {
            output.set_payload(self.number_of_executions);
        } else {
            output.clear_payload();
        }
        self.number_of_executions += 1;

        Ok(())
    }
}

pub type StrSinkTask = SinkTask<String>;
pub type IntSinkTask = SinkTask<i64>;

#[derive(Default)]
pub struct SinkTask<T> {
    _marker: PhantomData<T>,
}

impl<T> Freezable for SinkTask<T> {}

impl<'cl, T> CuSinkTask<'cl> for SinkTask<T>
where
    T: CuMsgPayload + 'cl + 'static + Display,
{
    type Input = input_msg!('cl, T);

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
