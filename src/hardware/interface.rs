
/// A generic hardware trait that could be implemented by Razer, Corsair, etc.
pub trait DeviceHardware: Send + Sync {
    /// Sends a raw byte payload to the specified device ID
    fn send_report(&self, pid: u16, payload: &[u8]) -> Result<(), &'static str>;
}
