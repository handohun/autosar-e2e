//! # E2E Profile 4M Implementation
//!
//! Profile 4M is identical to Profile 4 but includes additional fields
//! in CRC calculation: message_type, message_result, and source_id

use crate::profiles::profile4::{Profile4, Profile4Config}; // Reuse Profile4Config
use crate::{E2EProfile, E2EResult, E2EStatus};

const BITS_PER_BYTE : u16 = 8;

/// Check Item for E2E Profile 4
#[derive(Debug, Clone)]
pub struct Profile4mCheck {
    rx_source_id: u32,
    rx_message_type: u8,
    rx_message_result: u8,
}

/// E2E Profile 4m Implementation - minimal code by reusing Profile4 logic
#[derive(Clone)]
pub struct Profile4m {
    base: Profile4,
    config: Profile4Config,
    pub message_type: u8,
    pub message_result: u8,
    pub source_id: u32,
}

impl Profile4m {
    fn write_source_id(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset+12..=offset+15].copy_from_slice(&self.source_id.to_be_bytes());
    }
    fn write_message_type(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 12] = (data[offset + 12] & 0x3F) | ((self.message_type & 0x03) << 6);
    }
    fn write_message_result(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 12] = (data[offset + 12] & 0xCF) | ((self.message_result & 0x03) << 4);
    }
    fn read_source_id(&self, data: &[u8]) -> u32 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([data[offset + 12], data[offset + 13], data[offset + 14], data[offset + 15]]) & 0x0FFFFFFF
    }
    fn read_message_type(&self, data: &[u8]) -> u8 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        (data[offset + 12] >> 6) & 0x03
    }
    fn read_message_result(&self, data: &[u8]) -> u8 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        (data[offset + 12] >> 4) & 0x03
    }
    fn do_checks(&mut self, check_items: Profile4mCheck) -> E2EStatus {
        if self.source_id != check_items.rx_source_id {
            return E2EStatus::SourceIdError;
        }
        if self.message_result != check_items.rx_message_result {
            return E2EStatus::MessageResultError;
        }
        if self.message_type != check_items.rx_message_type {
            return E2EStatus::MessageTypeError;
        }
        E2EStatus::Ok
    }
}

impl E2EProfile for Profile4m {
    type Config = Profile4Config;

    fn new(config: Self::Config) -> Self {
        // Validate using Profile4's validation
        Self {
            base: crate::profiles::profile4::Profile4::new(config.clone()), // This validates config
            config,
            message_type: 0x00,
            message_result: 0x00,
            source_id: 0x0a0b0c0d,
        }
    }

    fn protect(&mut self, data: &mut [u8]) -> E2EResult<()> {
        // Write Profile4m specific fields first
        self.write_source_id(data);
        self.write_message_result(data);
        self.write_message_type(data);
        self.base.protect(data)?;
        Ok(())
    }

    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus> {
        let mut status = self.base.check(data)?;
        let check_items = Profile4mCheck {
            rx_source_id: self.read_source_id(data),
            rx_message_result: self.read_message_result(data),
            rx_message_type: self.read_message_type(data),
        };
        if (status == E2EStatus::Ok) || (status == E2EStatus::OkSomeLost) {
            status = self.do_checks(check_items);
        }
        Ok(status)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_profile4m_basic_request_example() {
        let mut profile_tx = Profile4m::new(Profile4Config::default());
        let mut profile_rx = Profile4m::new(Profile4Config::default());

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.source_id = 0x00123456;
        profile_tx.message_result = 0;
        profile_tx.message_type = 0;
        profile_tx.protect(&mut data).unwrap();
        // length check
        assert_eq!(data[0], 0x00);
        assert_eq!(data[1], 0x14);
        // counter check
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x00);
        // data id check
        assert_eq!(data[4], 0x0a);
        assert_eq!(data[5], 0x0b);
        assert_eq!(data[6], 0x0c);
        assert_eq!(data[7], 0x0d);
        // crc check
        assert_eq!(data[8], 0xae);
        assert_eq!(data[9], 0x67);
        assert_eq!(data[10], 0x4c);
        assert_eq!(data[11], 0xa0);
        // data check
        assert_eq!(data[12], 0x00);
        assert_eq!(data[13], 0x12);
        assert_eq!(data[14], 0x34);
        assert_eq!(data[15], 0x56);
        profile_rx.source_id = 0x00123456;
        profile_rx.message_result = 0;
        profile_rx.message_type = 0;
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
    #[test]
    fn test_profile4m_basic_response_example() {
        let mut profile_tx = Profile4m::new(Profile4Config::default());
        let mut profile_rx = Profile4m::new(Profile4Config::default());

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.source_id = 0x00123456;
        profile_tx.message_result = 0;
        profile_tx.message_type = 1;
        profile_tx.protect(&mut data).unwrap();
        // length check
        assert_eq!(data[0], 0x00);
        assert_eq!(data[1], 0x14);
        // counter check
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x00);
        // data id check
        assert_eq!(data[4], 0x0a);
        assert_eq!(data[5], 0x0b);
        assert_eq!(data[6], 0x0c);
        assert_eq!(data[7], 0x0d);
        // crc check
        assert_eq!(data[8], 0x85);
        assert_eq!(data[9], 0x25);
        assert_eq!(data[10], 0x76);
        assert_eq!(data[11], 0x19);
        // data check
        assert_eq!(data[12], 0x40);
        assert_eq!(data[13], 0x12);
        assert_eq!(data[14], 0x34);
        assert_eq!(data[15], 0x56);
        profile_rx.source_id = 0x00123456;
        profile_rx.message_result = 0;
        profile_rx.message_type = 1;
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
    #[test]
    fn test_profile4m_basic_error_example() {
        let mut profile_tx = Profile4m::new(Profile4Config::default());
        let mut profile_rx = Profile4m::new(Profile4Config::default());

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.source_id = 0x00123456;
        profile_tx.message_result = 1;
        profile_tx.message_type = 1;
        profile_tx.protect(&mut data).unwrap();
        // length check
        assert_eq!(data[0], 0x00);
        assert_eq!(data[1], 0x14);
        // counter check
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x00);
        // data id check
        assert_eq!(data[4], 0x0a);
        assert_eq!(data[5], 0x0b);
        assert_eq!(data[6], 0x0c);
        assert_eq!(data[7], 0x0d);
        // crc check
        assert_eq!(data[8], 0x23);
        assert_eq!(data[9], 0x45);
        assert_eq!(data[10], 0x57);
        assert_eq!(data[11], 0x0f);
        // data check
        assert_eq!(data[12], 0x50);
        assert_eq!(data[13], 0x12);
        assert_eq!(data[14], 0x34);
        assert_eq!(data[15], 0x56);
        profile_rx.source_id = 0x00123456;
        profile_rx.message_result = 1;
        profile_rx.message_type = 1;
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
}
