//! # E2E Profile 8 Implementation
//!
//! Profile 8 is designed for protecting large data packets
//! with low overhead. It uses:
//! - 32-bit CRC for data integrity
//! - 32-bit counter for sequence checking
//! - 32-bit Data ID for masquerade prevention
//! - 32-bit Data Length to support dynamic size data
//!
//! # Data layout
//! [DATA ... | CRC(4B) | LENGTH(4B) | CONTER(4B) | ID (4B) | DATA ...]
use crate::{
    counter::{Counter32, CounterOps},
    field_ops, validation,
};
use crate::{E2EProfile, E2EResult, E2EStatus};
use crc::{Crc, CRC_32_AUTOSAR};

// Constants
const BITS_PER_BYTE: u32 = 8;

/// Configuration for E2E Profile 8
#[derive(Debug, Clone)]
pub struct Profile8Config {
    /// data id
    pub data_id: u32,
    /// Bit offset of the first bit of the E2E header from the beginning of the Data
    pub offset: u32,
    /// Minimal length of Data, in bits
    pub min_data_length: u32,
    /// Maximal length of Data, in bits
    pub max_data_length: u32,
    /// Maximum allowed delta between consecutive counters
    pub max_delta_counter: u32,
}

/// Check Item for E2E Profile 8
#[derive(Debug, Clone)]
pub struct Profile8Check {
    rx_data_length: u32,
    rx_counter: u32,
    rx_data_id: u32,
    rx_crc: u32,
    calculated_crc: u32,
    data_len: u32,
}

impl Default for Profile8Config {
    fn default() -> Self {
        Self {
            data_id: 0x0a0b0c0d,
            offset: 0x00000000,
            min_data_length: 128,        // 16bytes
            max_data_length: 4294967295, // MAX(U32)
            max_delta_counter: 1,
        }
    }
}

/// E2E Profile 8 Implementation
///
/// Implements AUTOSAR E2E Profile 8 protection mechanism
#[derive(Clone)]
pub struct Profile8 {
    config: Profile8Config,
    counter: u32,
    initialized: bool,
}

impl Profile8 {
    /// Validate configuration parameters
    fn validate_config(config: &Profile8Config) -> E2EResult<()> {
        validation::validate_min_data_length_u32(config.min_data_length, 16 * BITS_PER_BYTE)?;
        validation::validate_max_data_length_u32(config.max_data_length, config.min_data_length)?;
        validation::validate_counter_config_u32(config.max_delta_counter)?;
        Ok(())
    }
    /// Validate data length against min/max constraints
    fn validate_length(&self, len: u32) -> E2EResult<()> {
        let min_bytes = self.config.min_data_length / BITS_PER_BYTE;
        let max_bytes = self.config.max_data_length / BITS_PER_BYTE;
        validation::validate_data_length_range_u32(len, min_bytes, max_bytes)
    }
    fn write_data_length(&self, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes_u32(self.config.offset);
        field_ops::write_be_u32_at(data, offset + 4, data.len() as u32);
    }
    fn write_counter(&self, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes_u32(self.config.offset);
        field_ops::write_be_u32_at(data, offset + 8, self.counter);
    }
    fn write_data_id(&self, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes_u32(self.config.offset);
        field_ops::write_be_u32_at(data, offset + 12, self.config.data_id);
    }
    fn compute_crc(&self, data: &[u8]) -> u32 {
        let crc: Crc<u32> = Crc::<u32>::new(&CRC_32_AUTOSAR);
        let mut digest = crc.digest();
        let offset = field_ops::calculate_offset_bytes_u32(self.config.offset);
        digest.update(&data[0..offset]); // crc calculation data before offset
        digest.update(&data[(offset + 4)..]); // crc calculation data after offset
        digest.finalize()
    }
    fn write_crc(&self, calculated_crc: u32, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes_u32(self.config.offset);
        field_ops::write_be_u32_at(data, offset, calculated_crc);
    }
    fn increment_counter(&mut self) {
        self.counter = Counter32::increment_counter(self.counter);
    }

    fn read_data_length(&self, data: &[u8]) -> u32 {
        let offset = field_ops::calculate_offset_bytes_u32(self.config.offset);
        field_ops::read_be_u32_at(data, offset + 4)
    }
    fn read_counter(&self, data: &[u8]) -> u32 {
        let offset = field_ops::calculate_offset_bytes_u32(self.config.offset);
        field_ops::read_be_u32_at(data, offset + 8)
    }
    fn read_data_id(&self, data: &[u8]) -> u32 {
        let offset = field_ops::calculate_offset_bytes_u32(self.config.offset);
        field_ops::read_be_u32_at(data, offset + 12)
    }
    fn read_crc(&self, data: &[u8]) -> u32 {
        let offset = field_ops::calculate_offset_bytes_u32(self.config.offset);
        field_ops::read_be_u32_at(data, offset)
    }

    fn do_checks(&mut self, check_items: Profile8Check) -> E2EStatus {
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
    fn validate_counter(&self, rx_counter: u32) -> E2EStatus {
        Counter32::validate_counter(
            self.counter,
            rx_counter,
            self.config.max_delta_counter,
            self.initialized,
        )
    }
}

impl E2EProfile for Profile8 {
    type Config = Profile8Config;

    fn new(config: Self::Config) -> Self {
        // Validate config (panic if invalid in constructor for simplicity)
        Self::validate_config(&config).expect("Invalid Profile8 configuration");
        Self {
            config,
            counter: 0,
            initialized: false,
        }
    }

    fn protect(&mut self, data: &mut [u8]) -> E2EResult<()> {
        self.validate_length(data.len() as u32)?;
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
        self.validate_length(data.len() as u32)?;
        let check_items = Profile8Check {
            rx_data_length: self.read_data_length(data),
            rx_counter: self.read_counter(data),
            rx_crc: self.read_crc(data),
            rx_data_id: self.read_data_id(data),
            calculated_crc: self.compute_crc(data),
            data_len: data.len() as u32,
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
        let mut profile_tx = Profile8::new(Profile8Config::default());
        let mut profile_rx = Profile8::new(Profile8Config::default());

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // CRC check
        assert_eq!(data[0], 0x41);
        assert_eq!(data[1], 0x49);
        assert_eq!(data[2], 0x4e);
        assert_eq!(data[3], 0x52);
        // length check
        assert_eq!(data[4], 0x00);
        assert_eq!(data[5], 0x00);
        assert_eq!(data[6], 0x00);
        assert_eq!(data[7], 0x14);
        // counter check
        assert_eq!(data[8], 0x00);
        assert_eq!(data[9], 0x00);
        assert_eq!(data[10], 0x00);
        assert_eq!(data[11], 0x00);
        // data id check
        assert_eq!(data[12], 0x0a);
        assert_eq!(data[13], 0x0b);
        assert_eq!(data[14], 0x0c);
        assert_eq!(data[15], 0x0d);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }

    #[test]
    fn test_profile8_offset_example() {
        let config = Profile8Config {
            offset: 64,
            ..Default::default()
        };

        let mut profile_tx = Profile8::new(config.clone());
        let mut profile_rx = Profile8::new(config);

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // CRC check
        assert_eq!(data[8], 0xe8);
        assert_eq!(data[9], 0x91);
        assert_eq!(data[10], 0xe5);
        assert_eq!(data[11], 0xa8);
        // length check
        assert_eq!(data[12], 0x00);
        assert_eq!(data[13], 0x00);
        assert_eq!(data[14], 0x00);
        assert_eq!(data[15], 0x1c);
        // counter check
        assert_eq!(data[16], 0x00);
        assert_eq!(data[17], 0x00);
        assert_eq!(data[18], 0x00);
        assert_eq!(data[19], 0x00);
        // data id check
        assert_eq!(data[20], 0x0a);
        assert_eq!(data[21], 0x0b);
        assert_eq!(data[22], 0x0c);
        assert_eq!(data[23], 0x0d);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
    #[test]
    fn test_profile8_counter_wraparound() {
        let config = Profile8Config {
            offset: 64,
            ..Default::default()
        };

        let mut profile_tx = Profile8::new(config.clone());
        let mut profile_rx = Profile8::new(config);

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[16], 0x00);
        assert_eq!(data[17], 0x00);
        assert_eq!(data[18], 0x00);
        assert_eq!(data[19], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[16], 0x00);
        assert_eq!(data[17], 0x00);
        assert_eq!(data[18], 0x00);
        assert_eq!(data[19], 0x01);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_rx.counter = 0xFFFFFFFE;
        profile_tx.counter = 0xFFFFFFFF;
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[16], 0xFF);
        assert_eq!(data[17], 0xFF);
        assert_eq!(data[18], 0xFF);
        assert_eq!(data[19], 0xFF);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[16], 0x00);
        assert_eq!(data[17], 0x00);
        assert_eq!(data[18], 0x00);
        assert_eq!(data[19], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
}
