use razermatter_lib::protocol::RazerPayload;

#[test]
fn test_create_color_payload() {
    let payload = RazerPayload::new_color(0x3F, 0x05, 255, 128, 64);
    
    assert_eq!(payload.data.len(), 91);
    
    // Check headers
    assert_eq!(payload.data[0], 0x00); // HID Report ID
    assert_eq!(payload.data[1], 0x00); // Status
    assert_eq!(payload.data[2], 0x3F); // Transaction ID
    
    // Check parameters
    assert_eq!(payload.data[6], 0x09); // Data size for color payload
    assert_eq!(payload.data[7], 0x0F); // Command class
    assert_eq!(payload.data[8], 0x02); // Command id
    
    // Check specific args
    assert_eq!(payload.data[9], 0x01); // VARSTORE
    assert_eq!(payload.data[10], 0x05); // led_id
    assert_eq!(payload.data[11], 0x01); // Static Effect
    assert_eq!(payload.data[15], 255); // R
    assert_eq!(payload.data[16], 128); // G
    assert_eq!(payload.data[17], 64); // B
    
    // Verify CRC (the last byte) is properly set. 
    // We can manually XOR the values here, or just trust that calculate_crc did it.
    let mut expected_crc = 0;
    for i in 3..89 {
        expected_crc ^= payload.data[i];
    }
    assert_eq!(payload.data[89], expected_crc);
}

#[test]
fn test_create_brightness_payload() {
    let payload = RazerPayload::new_brightness(0x1F, 0x00, 254);
    
    assert_eq!(payload.data.len(), 91);
    
    assert_eq!(payload.data[2], 0x1F); // Transaction ID
    assert_eq!(payload.data[6], 0x03); // Data size for brightness
    assert_eq!(payload.data[7], 0x0F); // Command class
    assert_eq!(payload.data[8], 0x04); // Command id
    
    // Check specific args
    assert_eq!(payload.data[9], 0x01); // VARSTORE
    assert_eq!(payload.data[10], 0x00); // led_id
    assert_eq!(payload.data[11], 254); // Level
}
