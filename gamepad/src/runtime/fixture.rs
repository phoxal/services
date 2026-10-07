//! Explicit fixture inputs driving the real generated adapter and owner.
use super::{Config, Gamepad, phoxal_runtime_gamepad::Adapter};
use crate::host::{Frame, Input};
use phoxal::runtime::{
    InitContext, RegisteredRuntime, Runtime, RuntimeSpec, StepContext, outputs::OutputBindings,
};
use std::sync::{Arc, Mutex};

type Inputs = <Adapter as Runtime>::Inputs;
type Outputs = <Adapter as Runtime>::Outputs;

struct Fixture {
    adapter: Adapter,
    input: Box<dyn Fn() -> Box<dyn Input> + Send + Sync>,
}
impl Fixture {
    fn new(input: impl Fn() -> Box<dyn Input> + Send + Sync + 'static) -> Self {
        Self {
            adapter: Adapter::new(),
            input: Box::new(input),
        }
    }
}
impl Runtime for Fixture {
    type Config = Config;
    type State = Gamepad;
    type Inputs = Inputs;
    type Outputs = Outputs;
    fn init(&self, _ctx: &InitContext, config: Config) -> phoxal::Result<Gamepad> {
        self.adapter.authoring.reset();
        Gamepad::with_input(config, || Ok((self.input)()))
    }
    fn step(
        &self,
        ctx: &StepContext,
        state: Gamepad,
        inputs: &Inputs,
    ) -> phoxal::Result<(Gamepad, Outputs)> {
        self.adapter.step(ctx, state, inputs)
    }
    fn accepted(&self) {
        self.adapter.accepted();
    }
    fn discarded(&self) {
        self.adapter.discarded();
    }
}
impl OutputBindings for Fixture {
    const FIELDS: &'static [phoxal::runtime::outputs::OutputField] = Adapter::FIELDS;
    fn encode_transport(
        &self,
        state: &Gamepad,
        context: StepContext,
        resolve: &dyn Fn(&str) -> Option<phoxal::contracts::MethodSignature>,
        source: &str,
    ) -> phoxal::Result<Vec<phoxal::runtime::transport::PreparedOutput>> {
        self.adapter
            .encode_transport(state, context, resolve, source)
    }
}
impl RegisteredRuntime for Fixture {
    const SPEC: RuntimeSpec = Adapter::SPEC;
    fn retain_artifact_metadata() {
        Adapter::retain_artifact_metadata();
    }
}
struct SharedInput(Arc<Mutex<Frame>>);
impl Input for SharedInput {
    fn poll(&mut self, _: &Config) -> Frame {
        self.0.lock().expect("fixture input lock").clone()
    }
}

pub type LocalHarness = phoxal::runtime::Harness<Gamepad>;
pub fn harness(frame: Arc<Mutex<Frame>>) -> phoxal::Result<LocalHarness> {
    LocalHarness::with_runtime(
        Fixture::new(move || Box::new(SharedInput(frame.clone()))),
        Config::default(),
    )
}

#[cfg(test)]
mod initialization_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn fixture_initialization_and_reset_use_only_the_explicit_input_factory() {
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let frame = Arc::new(Mutex::new(Frame::default()));
        let fixture = Fixture::new(move || {
            observed.fetch_add(1, Ordering::SeqCst);
            Box::new(SharedInput(frame.clone()))
        });
        let mut runtime = LocalHarness::with_runtime(fixture, Config::default()).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        runtime.advance_to(std::time::Duration::ZERO).unwrap();
        runtime.reset(Config::default()).unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(runtime.intent().is_none());
    }

    #[test]
    fn shared_constructor_validates_before_input_and_propagates_factory_failure() {
        let calls = AtomicUsize::new(0);
        let mut invalid = Config::default();
        invalid.linear.scale = f64::NAN;
        let result = Gamepad::with_input(invalid, || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(SharedInput(Arc::new(
                Mutex::new(Frame::default()),
            ))))
        });
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        let result = Gamepad::with_input(Config::default(), || {
            Err(phoxal::anyhow!("fixture construction refused"))
        });
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains("fixture construction refused")
        );
    }
}
