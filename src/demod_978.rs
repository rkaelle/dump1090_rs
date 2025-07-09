// UAT demodulator for 978 MHz Universal Access Transceiver
// This module implements CPFSK demodulation for UAT signals

use crate::{MagnitudeBuffer, UatFrame, UatFrameType};
use num_complex::Complex;

const UAT_SYNC_WORD: u32 = 0x5A5A5A5A; // UAT sync word
const UAT_SYNC_BITS: usize = 32;

// UAT uses CPFSK modulation at 978 MHz
// Sample rate: 2.083334 MHz
// Symbol rate: 1.041667 MHz
// Samples per symbol: 2
const SAMPLES_PER_SYMBOL: usize = 2;

// UAT preamble detection
const UAT_PREAMBLE_LEN: usize = 32; // 32 bits
const UAT_PREAMBLE_PATTERN: [u8; 4] = [0x5A, 0x5A, 0x5A, 0x5A];

#[derive(Debug, Clone)]
pub struct UatDemodulator {
    sample_rate: f64,
    symbol_rate: f64,
    samples_per_symbol: usize,
    phase_accumulator: f64,
    last_sample: Complex<f32>,
}

impl UatDemodulator {
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            symbol_rate: 1_041_667.0,
            samples_per_symbol: (sample_rate / 1_041_667.0) as usize,
            phase_accumulator: 0.0,
            last_sample: Complex::new(0.0, 0.0),
        }
    }

    // Demodulate UAT CPFSK signal
    pub fn demodulate(&mut self, iq_samples: &[Complex<i16>]) -> Vec<UatFrame> {
        let mut frames = Vec::new();
        let mag_buffer = self.compute_magnitude(iq_samples);
        
        // Look for UAT sync patterns
        let mut i = 0;
        while i < mag_buffer.len() {
            if let Some(sync_pos) = self.find_sync_pattern(&mag_buffer[i..]) {
                i += sync_pos;
                
                // Try to decode frame starting at this position
                if let Some(frame) = self.decode_frame(&mag_buffer[i..]) {
                    i += self.get_frame_length(&frame);
                    frames.push(frame);
                } else {
                    i += 1;
                }
            } else {
                break;
            }
        }

        frames
    }

    // Convert I/Q samples to magnitude
    fn compute_magnitude(&self, iq_samples: &[Complex<i16>]) -> Vec<u16> {
        iq_samples.iter()
            .map(|sample| {
                let i = sample.re as f32;
                let q = sample.im as f32;
                ((i * i + q * q).sqrt() * 256.0) as u16
            })
            .collect()
    }

    // Find UAT sync pattern in magnitude buffer
    fn find_sync_pattern(&self, mag_buffer: &[u16]) -> Option<usize> {
        if mag_buffer.len() < UAT_PREAMBLE_LEN * SAMPLES_PER_SYMBOL {
            return None;
        }

        for i in 0..mag_buffer.len() - UAT_PREAMBLE_LEN * SAMPLES_PER_SYMBOL {
            if self.check_sync_pattern(&mag_buffer[i..]) {
                return Some(i);
            }
        }
        None
    }

    // Check if sync pattern matches at given position
    fn check_sync_pattern(&self, buffer: &[u16]) -> bool {
        let mut score = 0;
        let threshold = (buffer.iter().take(64).sum::<u16>() / 64) as f32 * 0.7;

        for bit in 0..UAT_SYNC_BITS {
            let sample_idx = bit * SAMPLES_PER_SYMBOL;
            if sample_idx >= buffer.len() {
                return false;
            }
            
            let sample_val = buffer[sample_idx] as f32;
            let expected_bit = (UAT_SYNC_WORD >> (31 - bit)) & 1;
            
            if (sample_val > threshold) == (expected_bit == 1) {
                score += 1;
            }
        }

        score > UAT_SYNC_BITS * 3 / 4 // 75% correlation threshold
    }

    // Decode a UAT frame
    fn decode_frame(&self, buffer: &[u16]) -> Option<UatFrame> {
        // Skip sync word
        let data_start = UAT_SYNC_BITS * SAMPLES_PER_SYMBOL;
        if buffer.len() < data_start + 432 { // Minimum frame size
            return None;
        }

        // Decode frame type bit
        let frame_type_bit = self.decode_bit(&buffer[data_start..]);
        let frame_type = if frame_type_bit == 0 {
            UatFrameType::Downlink
        } else {
            UatFrameType::Uplink
        };

        // Decode payload length
        let payload_len = self.decode_payload_length(&buffer[data_start + SAMPLES_PER_SYMBOL..]);
        
        // Decode payload
        let payload = self.decode_payload(
            &buffer[data_start + 8 * SAMPLES_PER_SYMBOL..],
            payload_len
        );

        // Create frame
        let raw_data = buffer[..data_start + (payload_len + 16) * SAMPLES_PER_SYMBOL].to_vec()
            .iter().map(|&x| (x >> 8) as u8).collect();
        
        Some(UatFrame::new(raw_data, frame_type, payload))
    }

    // Decode a single bit from samples
    fn decode_bit(&self, samples: &[u16]) -> u8 {
        if samples.len() < SAMPLES_PER_SYMBOL {
            return 0;
        }

        let sum: u32 = samples.iter().take(SAMPLES_PER_SYMBOL).map(|&x| x as u32).sum();
        let avg = sum / SAMPLES_PER_SYMBOL as u32;
        let threshold = 32768; // Adjust based on signal strength

        if avg > threshold { 1 } else { 0 }
    }

    // Decode payload length from frame header
    fn decode_payload_length(&self, samples: &[u16]) -> usize {
        let mut length = 0;
        for i in 0..7 { // 7 bits for length
            let bit = self.decode_bit(&samples[i * SAMPLES_PER_SYMBOL..]);
            length |= (bit as usize) << (6 - i);
        }
        length
    }

    // Decode payload data
    fn decode_payload(&self, samples: &[u16], length: usize) -> Vec<u8> {
        let mut payload = Vec::new();
        let _bits_needed = length * 8;
        
        for byte_idx in 0..length {
            let mut byte_val = 0u8;
            for bit_idx in 0..8 {
                let sample_idx = (byte_idx * 8 + bit_idx) * SAMPLES_PER_SYMBOL;
                if sample_idx >= samples.len() {
                    break;
                }
                let bit = self.decode_bit(&samples[sample_idx..]);
                byte_val |= bit << (7 - bit_idx);
            }
            payload.push(byte_val);
        }

        payload
    }

    // Get frame length in samples
    fn get_frame_length(&self, frame: &UatFrame) -> usize {
        frame.raw_data.len() / 2 // Approximate conversion from bytes to samples
    }
}

// Main demodulation function
pub fn demodulate978(mag: &MagnitudeBuffer) -> Result<Vec<UatFrame>, &'static str> {
    let mut demod = UatDemodulator::new(2_083_334.0);
    
    // Convert magnitude buffer to complex samples for processing
    let iq_samples: Vec<Complex<i16>> = mag.data[..mag.length]
        .chunks(2)
        .map(|chunk| {
            let i = chunk[0] as i16;
            let q = if chunk.len() > 1 { chunk[1] as i16 } else { 0 };
            Complex::new(i, q)
        })
        .collect();

    Ok(demod.demodulate(&iq_samples))
}

// UAT-specific message validation
pub fn validate_uat_message(frame: &UatFrame) -> bool {
    // Basic validation checks
    if frame.payload.is_empty() {
        return false;
    }

    // Check payload length constraints
    match frame.frame_type {
        UatFrameType::Downlink => {
            frame.payload.len() >= 18 && frame.payload.len() <= 34
        }
        UatFrameType::Uplink => {
            frame.payload.len() >= 18 && frame.payload.len() <= 424
        }
    }
}