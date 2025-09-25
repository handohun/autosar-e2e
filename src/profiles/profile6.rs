//! # E2E Profile 6 Implementation
//!
//! Profile 6 is designed for protecting large data packets
//! with low overhead. It uses:
//! - 16-bit CRC for data integrity
//! - 8-bit counter for sequence checking
//! - 16-bit Data ID for masquerade prevention
//! - 16-bit Data Length to support dynamic size data
//!
//! # Data layout
//! [DATA ... | CRC(2B) | LENGTH(2B) | Counter(1B) | DATA ...]
use crate::{
    counter::{Counter8, CounterOps},
    field_ops, validation,
};
use crate::{E2EError, E2EProfile, E2EResult, E2EStatus};
use crc::{Crc, CRC_16_IBM_3740};

// Constants
const BITS_PER_BYTE: u16 = 8;

/// Configuration for E2E Profile 6
#[derive(Debug, Clone)]
pub struct Profile6Config {
    /// data id
    pub data_id: u16,
    /// Bit offset of the first bit of the E2E header from the beginning of the Data
    pub offset: u16,
    /// Minimal length of Data, in bits
    pub min_data_length: u16,
    /// Maximal length of Data, in bits
    pub max_data_length: u16,
    /// Maximum allowed delta between consecutive counters
    pub max_delta_counter: u8,
}

/// Check Item for E2E Profile 6
#[derive(Debug, Clone)]
pub struct Profile6Check {
    rx_data_length: u16,
    rx_counter: u8,
    rx_crc: u16,
    calculated_crc: u16,
    data_len: u16,
}

impl Default for Profile6Config {
    fn default() -> Self {
        Self {
            data_id: 0x1234,
            offset: 0x0000,
            min_data_length: 40,    // 5bytes
            max_data_length: 32768, // 4096bytes
            max_delta_counter: 1,
        }
    }
}

/// E2E Profile 6 Implementation
///
/// Implements AUTOSAR E2E Profile 6 protection mechanism
#[derive(Clone)]
pub struct Profile6 {
    config: Profile6Config,
    counter: u8,
    initialized: bool,
}

impl Profile6 {
    /// Validate configuration parameters
    fn validate_config(config: &Profile6Config) -> E2EResult<()> {
        validation::validate_min_data_length(
            config.min_data_length,
            5 * BITS_PER_BYTE,
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
        validation::validate_counter_config_u8(config.max_delta_counter)?;
        Ok(())
    }
    /// Validate data length against min/max constraints
    fn validate_length(&self, len: u16) -> E2EResult<()> {
        let min_bytes = self.config.min_data_length / BITS_PER_BYTE;
        let max_bytes = self.config.max_data_length / BITS_PER_BYTE;
        validation::validate_data_length_range(len, min_bytes, max_bytes)
    }
    fn write_data_length(&self, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::write_be_u16_at(data, offset + 2, data.len() as u16);
    }
    fn write_counter(&self, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::write_be_u8_at(data, offset + 4, self.counter);
    }
    fn compute_crc(&self, data: &[u8]) -> u16 {
        let crc: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
        let mut digest = crc.digest();
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        digest.update(&data[0..offset]); // crc calculation data before offset
        digest.update(&data[(offset + 2)..]); // crc calculation data after offset
        digest.update(&self.config.data_id.to_be_bytes());
        digest.finalize()
    }
    fn write_crc(&self, calculated_crc: u16, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::write_be_u16_at(data, offset, calculated_crc);
    }
    fn increment_counter(&mut self) {
        self.counter = Counter8::increment_counter(self.counter);
    }

    fn read_data_length(&self, data: &[u8]) -> u16 {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::read_be_u16_at(data, offset + 2)
    }
    fn read_counter(&self, data: &[u8]) -> u8 {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::read_be_u8_at(data, offset + 4)
    }
    fn read_crc(&self, data: &[u8]) -> u16 {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::read_be_u16_at(data, offset)
    }

    fn do_checks(&mut self, check_items: Profile6Check) -> E2EStatus {
        if check_items.calculated_crc != check_items.rx_crc {
            return E2EStatus::CrcError;
        }
        if check_items.rx_data_length != check_items.data_len {
            return E2EStatus::DataLengthError;
        }
        let status = self.validate_counter(check_items.rx_counter);
        self.counter = check_items.rx_counter;
        status
    }
    fn validate_counter(&self, rx_counter: u8) -> E2EStatus {
        Counter8::validate_counter(
            self.counter,
            rx_counter,
            self.config.max_delta_counter,
            self.initialized,
        )
    }
}

impl E2EProfile for Profile6 {
    type Config = Profile6Config;

    fn new(config: Self::Config) -> Self {
        // Validate config (panic if invalid in constructor for simplicity)
        Self::validate_config(&config).expect("Invalid Profile6 configuration");
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
        let calculated_crc = self.compute_crc(data);
        self.write_crc(calculated_crc, data);
        self.increment_counter();
        Ok(())
    }

    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus> {
        // Check data length
        self.validate_length(data.len() as u16)?;
        let check_items = Profile6Check {
            rx_data_length: self.read_data_length(data),
            rx_counter: self.read_counter(data),
            rx_crc: self.read_crc(data),
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
    fn test_profile6_basic_example() {
        let mut profile_tx = Profile6::new(Profile6Config::default());
        let mut profile_rx = Profile6::new(Profile6Config::default());

        let mut data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        profile_tx.protect(&mut data).unwrap();
        // CRC check
        assert_eq!(data[0], 0xb1);
        assert_eq!(data[1], 0x55);
        // length check
        assert_eq!(data[2], 0x00);
        assert_eq!(data[3], 0x08);
        // counter check
        assert_eq!(data[4], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }

    #[test]
    fn test_profile6_offset_example() {
        let config = Profile6Config {
            offset: 64,
            ..Default::default()
        };

        let mut profile_tx = Profile6::new(config.clone());
        let mut profile_rx = Profile6::new(config);

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // crc check
        assert_eq!(data[8], 0x4e);
        assert_eq!(data[9], 0xb7);
        // length check
        assert_eq!(data[10], 0x00);
        assert_eq!(data[11], 0x10);
        // counter check
        assert_eq!(data[12], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
    #[test]
    fn test_profile6_counter_wraparound() {
        let mut profile_tx = Profile6::new(Profile6Config::default());
        let mut profile_rx = Profile6::new(Profile6Config::default());

        let mut data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[4], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[4], 0x01);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_rx.counter = 0xFE;
        profile_tx.counter = 0xFF;
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[4], 0xFF);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[4], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
    }
}
