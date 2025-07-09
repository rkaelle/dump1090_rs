// UAT Message structure and decoding
// This module handles UAT message types and payload decoding

use crate::UatFrame;

// UAT message types
#[derive(Debug, Clone, PartialEq)]
pub enum UatMessageType {
    AdsB,           // ADS-B message
    TisB,           // TIS-B message  
    FisB,           // FIS-B message
    Unknown(u8),    // Unknown message type
}

// UAT ADS-B message structure
#[derive(Debug, Clone)]
pub struct UatAdsB {
    pub message_type: u8,
    pub icao_address: u32,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: i32,
    pub velocity_ns: i16,
    pub velocity_ew: i16,
    pub track_heading: u16,
    pub emitter_category: u8,
    pub call_sign: String,
    pub emergency_priority: u8,
    pub capability_codes: u8,
    pub operational_modes: u8,
    pub sv_integrity_level: u8,
    pub sv_accuracy_position: u8,
    pub sv_accuracy_velocity: u8,
    pub timestamp: u64,
}

// UAT TIS-B message structure
#[derive(Debug, Clone)]
pub struct UatTisB {
    pub site_id: u8,
    pub message_type: u8,
    pub address: u32,
    pub address_type: u8,
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: i32,
    pub timestamp: u64,
}

// UAT FIS-B message structure
#[derive(Debug, Clone)]
pub struct UatFisB {
    pub product_id: u16,
    pub time_to_live: u8,
    pub service_status: u8,
    pub data: Vec<u8>,
}

// Main UAT message structure
#[derive(Debug, Clone)]
pub struct UatMessage {
    pub message_type: UatMessageType,
    pub payload: UatMessagePayload,
    pub signal_level: f64,
    pub timestamp: u64,
}

#[derive(Debug, Clone)]
pub enum UatMessagePayload {
    AdsB(UatAdsB),
    TisB(UatTisB),
    FisB(UatFisB),
    Raw(Vec<u8>),
}

impl UatMessage {
    pub fn from_frame(frame: &UatFrame) -> Result<Self, &'static str> {
        if frame.payload.is_empty() {
            return Err("Empty payload");
        }

        let message_type = determine_message_type(&frame.payload)?;
        let payload = match message_type {
            UatMessageType::AdsB => {
                UatMessagePayload::AdsB(decode_adsb_message(&frame.payload)?)
            }
            UatMessageType::TisB => {
                UatMessagePayload::TisB(decode_tisb_message(&frame.payload)?)
            }
            UatMessageType::FisB => {
                UatMessagePayload::FisB(decode_fisb_message(&frame.payload)?)
            }
            UatMessageType::Unknown(_) => {
                UatMessagePayload::Raw(frame.payload.clone())
            }
        };

        Ok(UatMessage {
            message_type,
            payload,
            signal_level: frame.signal_level,
            timestamp: 0, // TODO: Add proper timestamp
        })
    }
}

// Determine message type from payload
fn determine_message_type(payload: &[u8]) -> Result<UatMessageType, &'static str> {
    if payload.is_empty() {
        return Err("Empty payload");
    }

    // UAT message type is determined by the first few bits
    let msg_type = payload[0] >> 3; // Upper 5 bits
    
    match msg_type {
        0..=4 => Ok(UatMessageType::AdsB),
        10..=15 => Ok(UatMessageType::TisB),
        20..=25 => Ok(UatMessageType::FisB),
        _ => Ok(UatMessageType::Unknown(msg_type)),
    }
}

// Decode ADS-B message
fn decode_adsb_message(payload: &[u8]) -> Result<UatAdsB, &'static str> {
    if payload.len() < 18 {
        return Err("ADS-B payload too short");
    }

    let message_type = payload[0] >> 3;
    
    // Extract ICAO address (24 bits)
    let icao_address = ((payload[1] as u32) << 16) | 
                      ((payload[2] as u32) << 8) | 
                      (payload[3] as u32);

    // Extract latitude (24 bits, signed)
    let lat_raw = ((payload[4] as i32) << 16) | 
                  ((payload[5] as i32) << 8) | 
                  (payload[6] as i32);
    let latitude = if lat_raw & 0x800000 != 0 {
        (lat_raw | 0xFF000000u32 as i32) as f64 * 180.0 / 16777216.0
    } else {
        lat_raw as f64 * 180.0 / 16777216.0
    };

    // Extract longitude (24 bits, signed)
    let lon_raw = ((payload[7] as i32) << 16) | 
                  ((payload[8] as i32) << 8) | 
                  (payload[9] as i32);
    let longitude = if lon_raw & 0x800000 != 0 {
        (lon_raw | 0xFF000000u32 as i32) as f64 * 180.0 / 16777216.0
    } else {
        lon_raw as f64 * 180.0 / 16777216.0
    };

    // Extract altitude (12 bits)
    let altitude_raw = ((payload[10] as u16) << 4) | ((payload[11] as u16) >> 4);
    let altitude = if altitude_raw == 0 {
        0
    } else {
        (altitude_raw as i32 - 1) * 25 - 1000
    };

    // Extract velocities (12 bits each, signed)
    let velocity_ns = ((payload[12] as i16) << 4) | ((payload[13] as i16) >> 4);
    let velocity_ew = (((payload[13] as i16) & 0x0F) << 8) | (payload[14] as i16);

    // Extract track/heading (8 bits)
    let track_heading = ((payload[15] as u16) * 360) / 256;

    // Extract emitter category (8 bits)
    let emitter_category = payload[16];

    // Extract call sign (8 characters, 6 bits each)
    let mut call_sign = String::new();
    if payload.len() >= 24 {
        for i in 0..8 {
            let char_index = (i * 6) / 8;
            let bit_offset = (i * 6) % 8;
            if char_index + 1 < payload.len() {
                let char_val = ((payload[17 + char_index] as u16) << 8) | 
                              (payload[18 + char_index] as u16);
                let char_bits = (char_val >> (10 - bit_offset)) & 0x3F;
                call_sign.push(decode_call_sign_char(char_bits as u8));
            }
        }
    }

    Ok(UatAdsB {
        message_type,
        icao_address,
        latitude,
        longitude,
        altitude,
        velocity_ns,
        velocity_ew,
        track_heading,
        emitter_category,
        call_sign: call_sign.trim().to_string(),
        emergency_priority: 0,
        capability_codes: 0,
        operational_modes: 0,
        sv_integrity_level: 0,
        sv_accuracy_position: 0,
        sv_accuracy_velocity: 0,
        timestamp: 0,
    })
}

// Decode TIS-B message
fn decode_tisb_message(payload: &[u8]) -> Result<UatTisB, &'static str> {
    if payload.len() < 18 {
        return Err("TIS-B payload too short");
    }

    let site_id = payload[0] & 0x0F;
    let message_type = payload[1] >> 3;
    let address = ((payload[2] as u32) << 16) | 
                 ((payload[3] as u32) << 8) | 
                 (payload[4] as u32);
    let address_type = payload[5] >> 4;

    // Extract position similar to ADS-B
    let lat_raw = ((payload[6] as i32) << 16) | 
                  ((payload[7] as i32) << 8) | 
                  (payload[8] as i32);
    let latitude = if lat_raw & 0x800000 != 0 {
        (lat_raw | 0xFF000000u32 as i32) as f64 * 180.0 / 16777216.0
    } else {
        lat_raw as f64 * 180.0 / 16777216.0
    };

    let lon_raw = ((payload[9] as i32) << 16) | 
                  ((payload[10] as i32) << 8) | 
                  (payload[11] as i32);
    let longitude = if lon_raw & 0x800000 != 0 {
        (lon_raw | 0xFF000000u32 as i32) as f64 * 180.0 / 16777216.0
    } else {
        lon_raw as f64 * 180.0 / 16777216.0
    };

    let altitude_raw = ((payload[12] as u16) << 4) | ((payload[13] as u16) >> 4);
    let altitude = if altitude_raw == 0 {
        0
    } else {
        (altitude_raw as i32 - 1) * 25 - 1000
    };

    Ok(UatTisB {
        site_id,
        message_type,
        address,
        address_type,
        latitude,
        longitude,
        altitude,
        timestamp: 0,
    })
}

// Decode FIS-B message
fn decode_fisb_message(payload: &[u8]) -> Result<UatFisB, &'static str> {
    if payload.len() < 6 {
        return Err("FIS-B payload too short");
    }

    let product_id = ((payload[0] as u16) << 8) | (payload[1] as u16);
    let time_to_live = payload[2];
    let service_status = payload[3];
    let data = payload[4..].to_vec();

    Ok(UatFisB {
        product_id,
        time_to_live,
        service_status,
        data,
    })
}

// Decode call sign character (6-bit encoding)
fn decode_call_sign_char(char_bits: u8) -> char {
    match char_bits {
        0 => ' ',
        1..=26 => (b'A' + char_bits - 1) as char,
        48..=57 => (b'0' + char_bits - 48) as char,
        _ => '?',
    }
}

// Helper function to format UAT message for output
impl UatMessage {
    pub fn to_hex_string(&self) -> String {
        match &self.payload {
            UatMessagePayload::Raw(data) => {
                format!("*{};", hex::encode(data))
            }
            UatMessagePayload::AdsB(adsb) => {
                format!("UAT ADS-B: ICAO={:06X} Lat={:.6} Lon={:.6} Alt={} Call={}", 
                       adsb.icao_address, adsb.latitude, adsb.longitude, 
                       adsb.altitude, adsb.call_sign)
            }
            UatMessagePayload::TisB(tisb) => {
                format!("UAT TIS-B: Site={} Addr={:06X} Lat={:.6} Lon={:.6} Alt={}", 
                       tisb.site_id, tisb.address, tisb.latitude, 
                       tisb.longitude, tisb.altitude)
            }
            UatMessagePayload::FisB(fisb) => {
                format!("UAT FIS-B: Product={} TTL={} Status={} Data={}bytes", 
                       fisb.product_id, fisb.time_to_live, 
                       fisb.service_status, fisb.data.len())
            }
        }
    }
}