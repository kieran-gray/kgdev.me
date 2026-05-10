use super::effect::PendingEffect;
use super::envelope::EventEnvelope;

/// Read-only context passed to a policy: the event's envelope (so policies can
/// see sequence/log_position/timestamp) and the current aggregate state (for
/// invariants the event payload doesn't carry on its own).
pub struct PolicyContext<'a, A, E> {
    pub envelope: &'a EventEnvelope<E>,
    pub state: &'a A,
}

impl<'a, A, E> PolicyContext<'a, A, E> {
    pub fn new(envelope: &'a EventEnvelope<E>, state: &'a A) -> Self {
        Self { envelope, state }
    }
}

/// A policy is a pure function from `(event variant, context) -> Vec<PendingEffect>`.
///
/// Functions are `fn` pointers so policies can be declared as `static` slices,
/// kept next to their event in the domain layer, and unit-tested without mocks.
pub type PolicyFn<E, A, EnvE, R> =
    fn(&E, &PolicyContext<'_, A, EnvE>) -> Vec<PendingEffect<R>>;

/// Attach a static list of policies to an event variant struct.
///
/// Implement this on the inner-event struct (e.g. `DatasetGenerationRequested`),
/// not the enum, so each event keeps its reactions file-local and discoverable.
pub trait HasPolicies<A: 'static, EnvE: 'static, R: 'static>: Sized + 'static {
    fn policies() -> &'static [PolicyFn<Self, A, EnvE, R>];

    fn apply_policies(
        &self,
        ctx: &PolicyContext<'_, A, EnvE>,
    ) -> Vec<PendingEffect<R>> {
        Self::policies().iter().flat_map(|f| f(self, ctx)).collect()
    }
}
