// Queue size constants
// Keep queues small to surface backpressure quickly. Large queues just hide
// slow consumers while accumulating stale data and wasting memory.
pub(crate) const OGN_INTAKE_QUEUE_SIZE: usize = 200;
pub(crate) const BEAST_INTAKE_QUEUE_SIZE: usize = 200;
pub(crate) const SBS_INTAKE_QUEUE_SIZE: usize = 200;
pub(crate) const ENVELOPE_INTAKE_QUEUE_SIZE: usize = 200;
pub(crate) const OGN_INTAKE_WORKERS: usize = 200;
