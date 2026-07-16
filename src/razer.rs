pub use hidapi::{HidApi, HidDevice};

pub const RAZER_VID: u16 = 0x1532;
pub const DOCK_PID: u16 = 0x0F21;
pub const KBD_PID: u16 = 0x0243;

fn calculate_crc(report: &[u8]) -> u8 {
    let mut crc = 0;
    // XOR bytes 3 to 88 (indices 3..89) since report[0] is hidapi ID and report[1..91] is the razer payload
    for i in 3..89 {
        crc ^= report[i];
    }
    crc
}

fn create_base_report(transaction_id: u8, data_size: u8, command_class: u8, command_id: u8) -> [u8; 91] {
    let mut report = [0u8; 91];
    report[0] = 0x00; // hidapi Report ID
    report[1] = 0x00; // Status: Request
    report[2] = transaction_id; // Transaction ID
    report[3] = 0x00; // Remaining packets (High)
    report[4] = 0x00; // Remaining packets (Low)
    report[5] = 0x00; // Protocol Type
    report[6] = data_size;
    report[7] = command_class;
    report[8] = command_id;
    report
}

pub fn set_device_lighting(pid: u16, transaction_id: u8, on: bool) -> Result<(), &'static str> {
    set_device_brightness(pid, transaction_id, if on { 255 } else { 0 })
}

pub fn set_device_brightness(pid: u16, transaction_id: u8, level: u8) -> Result<(), &'static str> {
    let api = HidApi::new().map_err(|_| "Failed to initialize HID API")?;
    let device = api.open(RAZER_VID, pid).map_err(|_| "Failed to open Razer Device")?;
    
    let mut br_report = create_base_report(transaction_id, 0x03, 0x0F, 0x04);
    let br_args = &mut br_report[9..89];
    br_args[0] = 0x01; // VARSTORE
    br_args[1] = 0x00; // ZERO_LED
    br_args[2] = level;
    br_report[89] = calculate_crc(&br_report);
    device.send_feature_report(&br_report).map_err(|_| "Failed to send brightness report")?;

    Ok(())
}

pub fn set_device_color(pid: u16, transaction_id: u8, r: u8, g: u8, b: u8) -> Result<(), &'static str> {
    let api = HidApi::new().map_err(|_| "Failed to initialize HID API")?;
    let device = api.open(RAZER_VID, pid).map_err(|_| "Failed to open Razer Device")?;
    
    let mut report = create_base_report(transaction_id, 0x09, 0x0F, 0x02);
    let args = &mut report[9..89];
    args[0] = 0x01; // VARSTORE
    args[1] = 0x00; // ZERO_LED
    args[2] = 0x01; // Static Effect
    args[5] = 0x01;
    args[6] = r;
    args[7] = g;
    args[8] = b;
    
    report[89] = calculate_crc(&report);
    device.send_feature_report(&report).map_err(|_| "Failed to send feature report")?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_base_report() {
        let report = create_base_report(0x1F, 0x05, 0x0F, 0x02);
        
        assert_eq!(report.len(), 91);
        assert_eq!(report[0], 0x00); // hidapi Report ID
        assert_eq!(report[1], 0x00); // Status Request
        assert_eq!(report[2], 0x1F); // Transaction ID for Thunderbolt 4 Dock
        assert_eq!(report[6], 0x05); // Data size
        assert_eq!(report[7], 0x0F); // Command class
        assert_eq!(report[8], 0x02); // Command id
    }

    #[test]
    fn test_calculate_crc() {
        let mut report = [0u8; 91];
        
        // Let's set some dummy data in the relevant payload range (indices 3..89)
        report[3] = 0xAA;
        report[4] = 0x55;
        report[5] = 0x01;
        report[88] = 0x10;
        
        // Expected CRC is the XOR of all these bytes
        let expected_crc = 0xAA ^ 0x55 ^ 0x01 ^ 0x10;
        assert_eq!(calculate_crc(&report), expected_crc);
    }
}
