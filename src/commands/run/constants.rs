// Queue size constants
pub(crate) const OGN_INTAKE_QUEUE_SIZE: usize = 5000;
pub(crate) const BEAST_INTAKE_QUEUE_SIZE: usize = 1000;
pub(crate) const SBS_INTAKE_QUEUE_SIZE: usize = 1000;
pub(crate) const AIRCRAFT_QUEUE_SIZE: usize = 5000;
pub(crate) const RECEIVER_STATUS_QUEUE_SIZE: usize = 50;
pub(crate) const RECEIVER_POSITION_QUEUE_SIZE: usize = 50;
pub(crate) const SERVER_STATUS_QUEUE_SIZE: usize = 50;
pub(crate) const ENVELOPE_INTAKE_QUEUE_SIZE: usize = 5_000;
pub(crate) const PACKET_ROUTER_WORKERS: usize = 50;

pub(crate) fn queue_warning_threshold(queue_size: usize) -> usize {
    queue_size / 2
}
