pub struct RazerPayload {
    pub data: [u8; 91],
}

impl RazerPayload {
    pub fn new_brightness(transaction_id: u8, led_id: u8, level: u8) -> Self {
        let mut report = Self::create_base_report(transaction_id, 0x03, 0x0F, 0x04);
        let args = &mut report.data[9..89];
        args[0] = 0x01; // VARSTORE
        args[1] = led_id;
        args[2] = level;
        report.finalize();
        report
    }

    pub fn new_color(transaction_id: u8, led_id: u8, r: u8, g: u8, b: u8) -> Self {
        let mut report = Self::create_base_report(transaction_id, 0x09, 0x0F, 0x02);
        let args = &mut report.data[9..89];
        args[0] = 0x01; // VARSTORE
        args[1] = led_id;
        args[2] = 0x01; // Static Effect
        args[5] = 0x01;
        args[6] = r;
        args[7] = g;
        args[8] = b;
        report.finalize();
        report
    }

    fn create_base_report(transaction_id: u8, data_size: u8, command_class: u8, command_id: u8) -> Self {
        let mut report = [0u8; 91];
        report[0] = 0x00; // hidapi Report ID
        report[1] = 0x00; // Status: Request
        report[2] = transaction_id;
        report[3] = 0x00; 
        report[4] = 0x00; 
        report[5] = 0x00; 
        report[6] = data_size;
        report[7] = command_class;
        report[8] = command_id;
        Self { data: report }
    }

    fn finalize(&mut self) {
        let mut crc = 0;
        for i in 3..89 {
            crc ^= self.data[i];
        }
        self.data[89] = crc;
    }
}
