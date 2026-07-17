pub mod razer;

use crate::protocol::RazerPayload;

/// A generic hardware trait that could be implemented by Razer, Corsair, etc.
pub trait RazerHardware: Send + Sync {
    fn send_report(&self, pid: u16, payload: &RazerPayload) -> Result<(), &'static str>;
}
