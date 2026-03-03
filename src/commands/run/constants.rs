// Queue size constants
// Queues should be large enough to absorb brief processing stalls (a few seconds
// of traffic) without causing upstream disconnections, but small enough that
// sustained backpressure is surfaced quickly rather than hidden.
// At ~350 msg/s OGN + ~400 msg/s Beast, 2000 gives ~3-6 seconds of buffer.
pub(crate) const OGN_INTAKE_QUEUE_SIZE: usize = 2000;
pub(crate) const BEAST_INTAKE_QUEUE_SIZE: usize = 2000;
pub(crate) const SBS_INTAKE_QUEUE_SIZE: usize = 2000;
pub(crate) const ENVELOPE_INTAKE_QUEUE_SIZE: usize = 2000;
// Worker counts reduced from 200: with batched raw_message INSERTs and batched
// aircraft position UPDATEs, each worker does far less DB work per message,
// so fewer workers can sustain the same throughput with less pool contention.
pub(crate) const OGN_INTAKE_WORKERS: usize = 50;
