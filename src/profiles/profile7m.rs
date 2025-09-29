//! # E2E Profile 7M Implementation
//!
//! Profile 7M is identical to Profile 7 but includes additional fields
//! in CRC calculation: message_type, message_result, and source_id

use crate::profile7::{Profile7, Profile7Config}; // Reuse Profile7Config
use crate::{E2EProfile, E2EResult, E2EStatus};

const BITS_PER_BYTE: u32 = 8;

/// Check Item for E2E Profile 7
#[derive(Debug, Clone)]
pub struct Profile7mCheck {
    rx_source_id: u32,
    rx_message_type: u8,
    rx_message_result: u8,
}

/// E2E Profile 7m Implementation - minimal code by reusing Profile7 logic
#[derive(Clone)]
pub struct Profile7m {
    base: Profile7,
    config: Profile7Config,
    pub message_type: u8,
    pub message_result: u8,
    pub source_id: u32,
}

impl Profile7m {
    fn write_source_id(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 20..=offset + 23].copy_from_slice(&self.source_id.to_be_bytes());
    }
    fn write_message_type(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 20] = (data[offset + 20] & 0x3F) | ((self.message_type & 0x03) << 6);
    }
    fn write_message_result(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 20] = (data[offset + 20] & 0xCF) | ((self.message_result & 0x03) << 4);
    }
    fn read_source_id(&self, data: &[u8]) -> u32 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([
            data[offset + 20],
            data[offset + 21],
            data[offset + 22],
            data[offset + 23],
        ]) & 0x0FFFFFFF
    }
    fn read_message_type(&self, data: &[u8]) -> u8 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        (data[offset + 20] >> 6) & 0x03
    }
    fn read_message_result(&self, data: &[u8]) -> u8 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        (data[offset + 20] >> 4) & 0x03
    }
    fn do_checks(&mut self, check_items: Profile7mCheck) -> E2EStatus {
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

impl E2EProfile for Profile7m {
    type Config = Profile7Config;

    fn new(config: Self::Config) -> E2EResult<Self> {
        // Validate using Profile7's validation
        let base = crate::profile7::Profile7::new(config.clone())?; // This validates config
        Ok(Self {
            base,
            config,
            message_type: 0x00,
            message_result: 0x00,
            source_id: 0x0a0b0c0d,
        })
    }

    fn protect(&mut self, data: &mut [u8]) -> E2EResult<()> {
        // Write Profile7m specific fields first
        self.write_source_id(data);
        self.write_message_result(data);
        self.write_message_type(data);
        self.base.protect(data)?;
        Ok(())
    }

    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus> {
        let mut status = self.base.check(data)?;
        let check_items = Profile7mCheck {
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
    fn test_profile7m_basic_request_example() {
        let config = Profile7Config {
            min_data_length: 192,
            ..Default::default()
        };

        let mut profile_tx = Profile7m::new(config.clone()).unwrap();
        let mut profile_rx = Profile7m::new(config).unwrap();

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.source_id = 0x00123456;
        profile_tx.message_result = 0;
        profile_tx.message_type = 0;
        profile_tx.protect(&mut data).unwrap();
        // CRC check
        assert_eq!(data[0], 0xae);
        assert_eq!(data[1], 0x96);
        assert_eq!(data[2], 0xa7);
        assert_eq!(data[3], 0xd0);
        assert_eq!(data[4], 0xa5);
        assert_eq!(data[5], 0x01);
        assert_eq!(data[6], 0x75);
        assert_eq!(data[7], 0x94);
        // length check
        assert_eq!(data[8], 0x00);
        assert_eq!(data[9], 0x00);
        assert_eq!(data[10], 0x00);
        assert_eq!(data[11], 0x1c);
        // counter check
        assert_eq!(data[12], 0x00);
        assert_eq!(data[13], 0x00);
        assert_eq!(data[14], 0x00);
        assert_eq!(data[15], 0x00);
        // data id check
        assert_eq!(data[16], 0x0a);
        assert_eq!(data[17], 0x0b);
        assert_eq!(data[18], 0x0c);
        assert_eq!(data[19], 0x0d);
        // message type/result/source id check
        assert_eq!(data[20], 0x00);
        assert_eq!(data[21], 0x12);
        assert_eq!(data[22], 0x34);
        assert_eq!(data[23], 0x56);
        profile_rx.source_id = 0x00123456;
        profile_rx.message_result = 0;
        profile_rx.message_type = 0;
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }

    #[test]
    fn test_profile7m_basic_response_example() {
        let config = Profile7Config {
            min_data_length: 192,
            ..Default::default()
        };

        let mut profile_tx = Profile7m::new(config.clone()).unwrap();
        let mut profile_rx = Profile7m::new(config).unwrap();

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.source_id = 0x00123456;
        profile_tx.message_result = 0;
        profile_tx.message_type = 1;
        profile_tx.protect(&mut data).unwrap();
        // CRC check
        assert_eq!(data[0], 0xa6);
        assert_eq!(data[1], 0x2d);
        assert_eq!(data[2], 0x64);
        assert_eq!(data[3], 0x86);
        assert_eq!(data[4], 0xe8);
        assert_eq!(data[5], 0x3f);
        assert_eq!(data[6], 0x2c);
        assert_eq!(data[7], 0xaf);
        // length check
        assert_eq!(data[8], 0x00);
        assert_eq!(data[9], 0x00);
        assert_eq!(data[10], 0x00);
        assert_eq!(data[11], 0x1c);
        // counter check
        assert_eq!(data[12], 0x00);
        assert_eq!(data[13], 0x00);
        assert_eq!(data[14], 0x00);
        assert_eq!(data[15], 0x00);
        // data id check
        assert_eq!(data[16], 0x0a);
        assert_eq!(data[17], 0x0b);
        assert_eq!(data[18], 0x0c);
        assert_eq!(data[19], 0x0d);
        // message type/result/source id check
        assert_eq!(data[20], 0x40);
        assert_eq!(data[21], 0x12);
        assert_eq!(data[22], 0x34);
        assert_eq!(data[23], 0x56);
        profile_rx.source_id = 0x00123456;
        profile_rx.message_result = 0;
        profile_rx.message_type = 1;
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }

    #[test]
    fn test_profile7m_basic_error_example() {
        let config = Profile7Config {
            min_data_length: 192,
            ..Default::default()
        };

        let mut profile_tx = Profile7m::new(config.clone()).unwrap();
        let mut profile_rx = Profile7m::new(config).unwrap();

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.source_id = 0x00123456;
        profile_tx.message_result = 1;
        profile_tx.message_type = 1;
        profile_tx.protect(&mut data).unwrap();
        // CRC check
        assert_eq!(data[0], 0x09);
        assert_eq!(data[1], 0xd9);
        assert_eq!(data[2], 0xe8);
        assert_eq!(data[3], 0x0c);
        assert_eq!(data[4], 0x47);
        assert_eq!(data[5], 0x34);
        assert_eq!(data[6], 0x32);
        assert_eq!(data[7], 0x02);
        // length check
        assert_eq!(data[8], 0x00);
        assert_eq!(data[9], 0x00);
        assert_eq!(data[10], 0x00);
        assert_eq!(data[11], 0x1c);
        // counter check
        assert_eq!(data[12], 0x00);
        assert_eq!(data[13], 0x00);
        assert_eq!(data[14], 0x00);
        assert_eq!(data[15], 0x00);
        // data id check
        assert_eq!(data[16], 0x0a);
        assert_eq!(data[17], 0x0b);
        assert_eq!(data[18], 0x0c);
        assert_eq!(data[19], 0x0d);
        // message type/result/source id check
        assert_eq!(data[20], 0x50);
        assert_eq!(data[21], 0x12);
        assert_eq!(data[22], 0x34);
        assert_eq!(data[23], 0x56);
        profile_rx.source_id = 0x00123456;
        profile_rx.message_result = 1;
        profile_rx.message_type = 1;
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
}
