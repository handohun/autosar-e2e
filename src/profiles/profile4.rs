//! # E2E Profile 4 Implementation
//!
//! Profile 4 is designed for protecting large data packets
//! with low overhead. It uses:
//! - 32-bit CRC for data integrity
//! - 16-bit counter for sequence checking
//! - 32-bit Data ID for masquerade prevention
//! - 16-bit Data Length to support dynamic size data
//!
//! # Data layout
//! [DATA ... | LENGTH(2B) | COUNTER(2B) | ID (4B) | CRC(4B) | DATA ...]
use crate::{
    counter::{Counter16, CounterOps},
    field_ops, validation,
};
use crate::{E2EError, E2EProfile, E2EResult, E2EStatus};
use crc::{Crc, CRC_32_AUTOSAR};

// Constants
const BITS_PER_BYTE: u16 = 8;

/// Configuration for E2E Profile 4
#[derive(Debug, Clone)]
pub struct Profile4Config {
    /// data id
    pub data_id: u32,
    /// Bit offset of the first bit of the E2E header from the beginning of the Data
    pub offset: u16,
    /// Minimal length of Data, in bits
    pub min_data_length: u16,
    /// Maximal length of Data, in bits
    pub max_data_length: u16,
    /// Maximum allowed delta between consecutive counters
    pub max_delta_counter: u16,
}

/// Check Item for E2E Profile 4
#[derive(Debug, Clone)]
pub struct Profile4Check {
    rx_data_length: u16,
    rx_counter: u16,
    rx_data_id: u32,
    rx_crc: u32,
    calculated_crc: u32,
    data_len: u16,
}

impl Default for Profile4Config {
    fn default() -> Self {
        Self {
            data_id: 0x0a0b0c0d,
            offset: 0x0000,
            min_data_length: 96,    // 12bytes
            max_data_length: 32768, // 4096bytes
            max_delta_counter: 1,
        }
    }
}

/// E2E Profile 4 Implementation
///
/// Implements AUTOSAR E2E Profile 4 protection mechanism
#[derive(Clone)]
pub struct Profile4 {
    config: Profile4Config,
    counter: u16,
    initialized: bool,
}

impl Profile4 {
    /// Validate configuration parameters
    fn validate_config(config: &Profile4Config) -> E2EResult<()> {
        validation::validate_min_data_length(
            config.min_data_length,
            12 * BITS_PER_BYTE,
            4096 * BITS_PER_BYTE,
        )?;
        validation::validate_max_data_length_u32(
            config.max_data_length as u32,
            config.min_data_length as u32,
        )?;
        if config.max_data_length > 4096 * 8 {
            return Err(E2EError::InvalidConfiguration(
                "Maximum Data length shall be at most 4096B".into(),
            ));
        }
        validation::validate_counter_config_u16(config.max_delta_counter)?;
        Ok(())
    }
    /// Validate data length against min/max constraints
    fn validate_length(&self, len: u16) -> E2EResult<()> {
        let min_bytes = self.config.min_data_length / BITS_PER_BYTE;
        let max_bytes = self.config.max_data_length / BITS_PER_BYTE;
        validation::validate_data_length_range(len, min_bytes, max_bytes)
    }
    fn increment_counter(&mut self) {
        self.counter = Counter16::increment_counter(self.counter);
    }
    fn write_data_length(&self, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::write_be_u16_at(data, offset, data.len() as u16);
    }
    fn write_counter(&self, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::write_be_u16_at(data, offset + 2, self.counter);
    }
    fn write_data_id(&self, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::write_be_u32_at(data, offset + 4, self.config.data_id);
    }
    fn compute_crc(&self, data: &[u8]) -> u32 {
        let crc: Crc<u32> = Crc::<u32>::new(&CRC_32_AUTOSAR);
        let mut digest = crc.digest();
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        digest.update(&data[0..offset + 8]); // crc calculation data before offset
        digest.update(&data[(offset + 12)..]); // crc calculation data after offset
        digest.finalize()
    }
    fn write_crc(&self, calculated_crc: u32, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::write_be_u32_at(data, offset + 8, calculated_crc);
    }
    fn read_data_length(&self, data: &[u8]) -> u16 {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::read_be_u16_at(data, offset)
    }
    fn read_counter(&self, data: &[u8]) -> u16 {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::read_be_u16_at(data, offset + 2)
    }
    fn read_data_id(&self, data: &[u8]) -> u32 {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::read_be_u32_at(data, offset + 4)
    }
    fn read_crc(&self, data: &[u8]) -> u32 {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::read_be_u32_at(data, offset + 8)
    }

    fn do_checks(&mut self, check_items: Profile4Check) -> E2EStatus {
        if check_items.calculated_crc != check_items.rx_crc {
            return E2EStatus::CrcError;
        }
        if check_items.rx_data_id != self.config.data_id {
            return E2EStatus::DataIdError;
        }
        if check_items.rx_data_length != check_items.data_len {
            return E2EStatus::DataLengthError;
        }
        let status = self.validate_counter(check_items.rx_counter);
        self.counter = check_items.rx_counter;
        status
    }
    fn validate_counter(&self, rx_counter: u16) -> E2EStatus {
        Counter16::validate_counter(
            self.counter,
            rx_counter,
            self.config.max_delta_counter,
            self.initialized,
        )
    }
}

impl E2EProfile for Profile4 {
    type Config = Profile4Config;

    fn new(config: Self::Config) -> Self {
        // Validate config (panic if invalid in constructor for simplicity)
        Self::validate_config(&config).expect("Invalid Profile4 configuration");
        Self {
            config,
            counter: 0,
            initialized: false,
        }
    }

    fn protect(&mut self, data: &mut [u8]) -> E2EResult<()> {
        self.validate_length(data.len() as u16)?;
        self.write_data_length(data);
        self.write_counter(data);
        self.write_data_id(data);
        let calculated_crc = self.compute_crc(data);
        self.write_crc(calculated_crc, data);
        self.increment_counter();
        Ok(())
    }

    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus> {
        // Check data length
        self.validate_length(data.len() as u16)?;
        let check_items = Profile4Check {
            rx_data_length: self.read_data_length(data),
            rx_counter: self.read_counter(data),
            rx_crc: self.read_crc(data),
            rx_data_id: self.read_data_id(data),
            calculated_crc: self.compute_crc(data),
            data_len: data.len() as u16,
        };
        let status = self.do_checks(check_items);
        if !self.initialized && matches!(status, E2EStatus::Ok | E2EStatus::OkSomeLost) {
            self.initialized = true;
        }
        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_profile4_basic_example() {
        let mut profile_tx = Profile4::new(Profile4Config::default());
        let mut profile_rx = Profile4::new(Profile4Config::default());

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // length check
        assert_eq!(data[0], 0x00);
        assert_eq!(data[1], 0x10);
        // counter check
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x00);
        // data id check
        assert_eq!(data[4], 0x0a);
        assert_eq!(data[5], 0x0b);
        assert_eq!(data[6], 0x0c);
        assert_eq!(data[7], 0x0d);
        // crc check
        assert_eq!(data[8], 0x86);
        assert_eq!(data[9], 0x2b);
        assert_eq!(data[10], 0x05);
        assert_eq!(data[11], 0x56);
        // data check
        assert_eq!(data[12], 0x00);
        assert_eq!(data[13], 0x00);
        assert_eq!(data[14], 0x00);
        assert_eq!(data[15], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }

    #[test]
    fn test_profile4_offset_example() {
        let config = Profile4Config {
            offset: 64,
            ..Default::default()
        };

        let mut profile_tx = Profile4::new(config.clone());
        let mut profile_rx = Profile4::new(config);

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // length check
        assert_eq!(data[8], 0x00);
        assert_eq!(data[9], 0x18);
        // counter check
        assert_eq!(data[10], 0x00);
        assert_eq!(data[11], 0x00);
        // data id check
        assert_eq!(data[12], 0x0a);
        assert_eq!(data[13], 0x0b);
        assert_eq!(data[14], 0x0c);
        assert_eq!(data[15], 0x0d);
        // crc check
        assert_eq!(data[16], 0x69);
        assert_eq!(data[17], 0xd7);
        assert_eq!(data[18], 0x50);
        assert_eq!(data[19], 0x2e);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
    #[test]
    fn test_profile4_counter_wraparound() {
        let config = Profile4Config {
            offset: 64,
            ..Default::default()
        };

        let mut profile_tx = Profile4::new(config.clone());
        let mut profile_rx = Profile4::new(config);

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[10], 0x00);
        assert_eq!(data[11], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        for i in 1u16..=0xFFFF {
            profile_tx.protect(&mut data).unwrap();
            // counter check
            assert_eq!(data[10], i.to_be_bytes()[0]);
            assert_eq!(data[11], i.to_be_bytes()[1]);
            assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        }
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[10], 0x00);
        assert_eq!(data[11], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
}
