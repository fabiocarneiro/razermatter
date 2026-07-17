pub use hidapi::HidApi;

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
    set_device_brightness(pid, transaction_id, 0x00, if on { 255 } else { 0 })
}

pub fn set_device_brightness(pid: u16, transaction_id: u8, led_id: u8, level: u8) -> Result<(), &'static str> {
    let api = HidApi::new().unwrap();
    let mut br_report = create_base_report(transaction_id, 0x03, 0x0F, 0x04);
    let br_args = &mut br_report[9..89];
    br_args[0] = 0x01; // VARSTORE
    br_args[1] = led_id;
    br_args[2] = level;
    br_report[89] = calculate_crc(&br_report);

    let paths: std::collections::HashSet<_> = api.device_list()
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
            if device.send_feature_report(&br_report).is_ok() {
                success = true;
                // DO NOT BREAK! Some interfaces (like the standard keyboard interface) 
                // might accept the report but do nothing. We must send to all paths!
            }
        }
    }

    if success {
        Ok(())
    } else {
        Err("Failed to send brightness report on any valid interface")
    }
}

pub fn set_device_color(pid: u16, transaction_id: u8, led_id: u8, r: u8, g: u8, b: u8) -> Result<(), &'static str> {
    let api = HidApi::new().unwrap();

    let mut report = create_base_report(transaction_id, 0x09, 0x0F, 0x02);
    let args = &mut report[9..89];
    args[0] = 0x01; // VARSTORE
    args[1] = led_id; 
    args[2] = 0x01; // Static Effect
    args[5] = 0x01;
    args[6] = r;
    args[7] = g;
    args[8] = b;
    
    report[89] = calculate_crc(&report);

    let paths: std::collections::HashSet<_> = api.device_list()
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
            if device.send_feature_report(&report).is_ok() {
                success = true;
                // DO NOT BREAK! 
            }
        }
    }

    if success {
        Ok(())
    } else {
        Err("Failed to send color feature report on any valid interface")
    }
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
