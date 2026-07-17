use hidapi::HidApi;
use std::collections::HashSet;
use crate::protocol::RazerPayload;

pub const RAZER_VID: u16 = 0x1532;
pub const DOCK_PID: u16 = 0x0F21;
pub const KBD_PID: u16 = 0x0243;

pub trait RazerHardware: Send + Sync {
    fn send_report(&self, pid: u16, payload: &RazerPayload) -> Result<(), &'static str>;
}

pub struct HidDeviceManager;

impl HidDeviceManager {
    pub fn new() -> Self {
        Self
    }
}

impl RazerHardware for HidDeviceManager {
    fn send_report(&self, pid: u16, payload: &RazerPayload) -> Result<(), &'static str> {
        let api = HidApi::new().map_err(|_| "Failed to initialize HID API")?;

        let paths: HashSet<_> = api.device_list()
            .filter(|d| d.vendor_id() == RAZER_VID && d.product_id() == pid)
            .filter(|d| {
                if pid == KBD_PID {
                    d.interface_number() == 2 // The Mouse interface exposes the proprietary protocol for Huntsman TE
                } else if pid == DOCK_PID {
                    d.usage_page() == 0x000C && d.usage() == 0x0001
                } else {
                    false
                }
            })
            .map(|d| d.path().to_owned())
            .collect();

        let mut success = false;
        for path in paths {
            if let Ok(device) = api.open_path(&path) {
                if device.send_feature_report(&payload.data).is_ok() {
                    success = true;
                }
            }
        }

        if success {
            Ok(())
        } else {
            Err("Failed to send feature report on any valid interface")
        }
    }
}
