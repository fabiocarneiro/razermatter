
/// A hardware trait for Razer devices
pub trait RazerHardware: Send + Sync {
    /// Sends a raw byte payload to the specified device ID
    fn send_report(&self, pid: u16, payload: &[u8]) -> Result<(), &'static str>;
}
