use hidapi::HidApi;
use std::thread::sleep;
use std::time::Duration;

const RAZER_VID: u16 = 0x1532;
const DOCK_PID: u16 = 0x0F21;

fn calculate_crc(report: &[u8]) -> u8 {
    let mut crc = 0;
    for i in 3..89 {
        crc ^= report[i];
    }
    crc
}

fn create_report(transaction_id: u8, data_size: u8, command_class: u8, command_id: u8, args: &[u8]) -> [u8; 91] {
    let mut report = [0u8; 91];
    report[0] = 0x00; // hidapi Report ID
    report[1] = 0x00; // Razer Status Request
    report[2] = transaction_id;
    report[6] = data_size;
    report[7] = command_class;
    report[8] = command_id;
    for (i, &b) in args.iter().enumerate() {
        report[9 + i] = b;
    }
    report[89] = calculate_crc(&report);
    report
}

fn send_report(device: &hidapi::HidDevice, name: &str, report: [u8; 91]) {
    println!("Sending: {}", name);
    match device.send_feature_report(&report) {
        Ok(_) => println!(" -> Success"),
        Err(e) => println!(" -> Failed: {:?}", e),
    }
    sleep(Duration::from_secs(3));
}

fn main() {
    let api = HidApi::new().expect("Failed to init HIDAPI");
    let device = api.open(RAZER_VID, DOCK_PID).expect("Failed to open Dock");

    println!("Testing combinations with 91-byte hidapi buffer...");

    send_report(&device, "Brightness 0 (0x1F)", create_report(0x1F, 0x03, 0x0F, 0x04, &[0x01, 0x00, 0x00]));
    send_report(&device, "Brightness 255 (0x1F)", create_report(0x1F, 0x03, 0x0F, 0x04, &[0x01, 0x00, 0xFF]));
    send_report(&device, "Effect NONE (0x1F)", create_report(0x1F, 0x06, 0x0F, 0x02, &[0x01, 0x00, 0x00]));
    send_report(&device, "Effect STATIC RED (0x1F)", create_report(0x1F, 0x09, 0x0F, 0x02, &[0x01, 0x00, 0x01, 0x00, 0x00, 0x01, 0xFF, 0x00, 0x00]));

    println!("Done.");
}
