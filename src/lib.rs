/*
This crate is a Rust implementation of dump978 UAT decoder.
It was developed to handle 978 MHz Universal Access Transceiver (UAT) ADS-B messages
based on the dump978 specifications and UAT protocol documentation.
*/

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

// public
pub mod demod_978;

// public(crate)
pub mod utils;

// private
mod fec;
pub mod uat_decode;
pub mod uat_message;

pub const UAT_MAG_BUF_SAMPLES: usize = 104_858; // Adjusted for 978 MHz sample rate
pub const UAT_SAMPLE_RATE: f64 = 2_083_334.0; // UAT sample rate

const TRAILING_SAMPLES: usize = 326;
pub const UAT_LONG_MSG_BYTES: usize = 34; // UAT long message size
pub const UAT_SHORT_MSG_BYTES: usize = 18; // UAT short message size

// UAT magnitude buffer structure
#[derive(Copy, Clone, Debug)]
pub struct MagnitudeBuffer {
    pub data: [u16; TRAILING_SAMPLES + UAT_MAG_BUF_SAMPLES],
    pub length: usize,
    pub first_sample_timestamp: usize,
}

impl Default for MagnitudeBuffer {
    fn default() -> Self {
        Self {
            data: [0_u16; TRAILING_SAMPLES + UAT_MAG_BUF_SAMPLES],
            length: 0,
            first_sample_timestamp: 0,
        }
    }
}

impl MagnitudeBuffer {
    pub fn push(&mut self, x: u16) {
        self.data[TRAILING_SAMPLES + self.length] = x;
        self.length += 1;
    }
}

// UAT frame structure
#[derive(Debug, Clone)]
pub struct UatFrame {
    pub raw_data: Vec<u8>,
    pub frame_type: UatFrameType,
    pub payload: Vec<u8>,
    pub rs_corrected: bool,
    pub signal_level: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UatFrameType {
    Downlink,
    Uplink,
}

impl UatFrame {
    pub fn new(raw_data: Vec<u8>, frame_type: UatFrameType, payload: Vec<u8>) -> Self {
        Self {
            raw_data,
            frame_type,
            payload,
            rs_corrected: false,
            signal_level: 0.0,
        }
    }
}
