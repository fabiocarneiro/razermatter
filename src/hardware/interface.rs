use crate::protocol::RazerPayload;

/// A generic hardware trait that could be implemented by Razer, Corsair, etc.
pub trait DeviceHardware: Send + Sync {
    // Note: We might eventually genericize RazerPayload if we add non-Razer devices,
    // but for now this trait defines the contract for sending payloads.
    fn send_report(&self, pid: u16, payload: &RazerPayload) -> Result<(), &'static str>;
}
