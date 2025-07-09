// Main UAT decoder module
// This module provides the main interface for decoding UAT frames

use crate::{UatFrame, fec, uat_message::UatMessage};

// UAT decoder statistics
#[derive(Debug, Default)]
pub struct UatStats {
    pub total_frames: u64,
    pub valid_frames: u64,
    pub corrected_frames: u64,
    pub adsb_frames: u64,
    pub tisb_frames: u64,
    pub fisb_frames: u64,
    pub error_frames: u64,
}

// Main UAT decoder
pub struct UatDecoder {
    pub stats: UatStats,
    enable_fec: bool,
}

impl UatDecoder {
    pub fn new() -> Self {
        Self {
            stats: UatStats::default(),
            enable_fec: true,
        }
    }

    pub fn new_with_fec(enable_fec: bool) -> Self {
        Self {
            stats: UatStats::default(),
            enable_fec,
        }
    }

    // Decode a UAT frame into a message
    pub fn decode_frame(&mut self, frame: &UatFrame) -> Result<UatMessage, &'static str> {
        self.stats.total_frames += 1;

        // Validate frame
        if !self.validate_frame(frame) {
            self.stats.error_frames += 1;
            return Err("Invalid frame");
        }

        // Apply FEC if enabled
        let mut corrected_frame = frame.clone();
        if self.enable_fec {
            match fec::correct_uat_frame(&mut corrected_frame) {
                Ok(true) => {
                    self.stats.corrected_frames += 1;
                    corrected_frame.rs_corrected = true;
                }
                Ok(false) => {
                    // No correction needed
                }
                Err(_) => {
                    self.stats.error_frames += 1;
                    return Err("FEC correction failed");
                }
            }
        }

        // Decode message
        let message = UatMessage::from_frame(&corrected_frame)?;
        
        // Update statistics
        self.stats.valid_frames += 1;
        match message.message_type {
            crate::uat_message::UatMessageType::AdsB => self.stats.adsb_frames += 1,
            crate::uat_message::UatMessageType::TisB => self.stats.tisb_frames += 1,
            crate::uat_message::UatMessageType::FisB => self.stats.fisb_frames += 1,
            crate::uat_message::UatMessageType::Unknown(_) => {}
        }

        Ok(message)
    }

    // Validate frame structure
    fn validate_frame(&self, frame: &UatFrame) -> bool {
        // Check minimum frame size
        if frame.payload.is_empty() {
            return false;
        }

        // Check frame type specific constraints
        match frame.frame_type {
            crate::UatFrameType::Downlink => {
                // Downlink frames: 18-34 bytes for ADS-B
                frame.payload.len() >= 18 && frame.payload.len() <= 34
            }
            crate::UatFrameType::Uplink => {
                // Uplink frames: 18-424 bytes for TIS-B/FIS-B
                frame.payload.len() >= 18 && frame.payload.len() <= 424
            }
        }
    }

    // Get decoder statistics
    pub fn get_stats(&self) -> &UatStats {
        &self.stats
    }

    // Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = UatStats::default();
    }
}

// Convenience function for decoding a single frame
pub fn decode_uat_frame(frame: &UatFrame) -> Result<UatMessage, &'static str> {
    let mut decoder = UatDecoder::new();
    decoder.decode_frame(frame)
}

// Convenience function for decoding multiple frames
pub fn decode_uat_frames(frames: &[UatFrame]) -> Vec<UatMessage> {
    let mut decoder = UatDecoder::new();
    let mut messages = Vec::new();

    for frame in frames {
        match decoder.decode_frame(frame) {
            Ok(message) => messages.push(message),
            Err(_) => continue, // Skip invalid frames
        }
    }

    messages
}

// UAT frame validation
pub fn validate_uat_frame(frame: &UatFrame) -> bool {
    // Check basic frame structure
    if frame.payload.is_empty() {
        return false;
    }

    // Check frame type specific constraints
    match frame.frame_type {
        crate::UatFrameType::Downlink => {
            frame.payload.len() >= 18 && frame.payload.len() <= 34
        }
        crate::UatFrameType::Uplink => {
            frame.payload.len() >= 18 && frame.payload.len() <= 424
        }
    }
}

// UAT message type detection
pub fn detect_message_type(payload: &[u8]) -> Result<crate::uat_message::UatMessageType, &'static str> {
    if payload.is_empty() {
        return Err("Empty payload");
    }

    let msg_type = payload[0] >> 3;
    match msg_type {
        0..=4 => Ok(crate::uat_message::UatMessageType::AdsB),
        10..=15 => Ok(crate::uat_message::UatMessageType::TisB),
        20..=25 => Ok(crate::uat_message::UatMessageType::FisB),
        _ => Ok(crate::uat_message::UatMessageType::Unknown(msg_type)),
    }
}

// UAT signal quality assessment
pub fn assess_signal_quality(frame: &UatFrame) -> f64 {
    // Basic signal quality assessment based on frame completeness
    let completeness = frame.payload.len() as f64 / 34.0; // Normalize to max downlink size
    let signal_strength = frame.signal_level;
    
    // Combine factors (this is a simplified assessment)
    (completeness * 0.7 + signal_strength * 0.3).min(1.0)
}

impl Default for UatDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl UatStats {
    pub fn success_rate(&self) -> f64 {
        if self.total_frames == 0 {
            0.0
        } else {
            self.valid_frames as f64 / self.total_frames as f64
        }
    }

    pub fn correction_rate(&self) -> f64 {
        if self.valid_frames == 0 {
            0.0
        } else {
            self.corrected_frames as f64 / self.valid_frames as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{UatFrame, UatFrameType};

    #[test]
    fn test_frame_validation() {
        let valid_frame = UatFrame::new(
            vec![0x5A, 0x5A, 0x5A, 0x5A],
            UatFrameType::Downlink,
            vec![0u8; 20], // Valid downlink payload size
        );
        assert!(validate_uat_frame(&valid_frame));

        let invalid_frame = UatFrame::new(
            vec![0x5A, 0x5A, 0x5A, 0x5A],
            UatFrameType::Downlink,
            vec![0u8; 10], // Too short for downlink
        );
        assert!(!validate_uat_frame(&invalid_frame));
    }

    #[test]
    fn test_message_type_detection() {
        // ADS-B message (type 0)
        let adsb_payload = vec![0x00, 0x01, 0x02, 0x03];
        assert_eq!(detect_message_type(&adsb_payload).unwrap(), 
                   crate::uat_message::UatMessageType::AdsB);

        // TIS-B message (type 10)
        let tisb_payload = vec![0x50, 0x01, 0x02, 0x03]; // 0x50 >> 3 = 10
        assert_eq!(detect_message_type(&tisb_payload).unwrap(), 
                   crate::uat_message::UatMessageType::TisB);
    }

    #[test]
    fn test_decoder_stats() {
        let mut decoder = UatDecoder::new();
        assert_eq!(decoder.get_stats().total_frames, 0);
        assert_eq!(decoder.get_stats().success_rate(), 0.0);
    }
}