//! # E2E Profile 5 Implementation
//!
//! Profile 5 is designed for protecting data packets
//! with low overhead. It uses:
//! - 16-bit CRC for data integrity
//! - 8-bit counter for sequence checking
//! - 8-bit Data ID for masquerade prevention
//!
//! # Data layout
//! [DATA ... | CRC(2B) | COUNTER(1B) | DATA ...]
use crate::{
    counter::{Counter8, CounterOps},
    field_ops, validation,
};
use crate::{E2EProfile, E2EResult, E2EStatus};
use crc::{Crc, CRC_16_IBM_3740};

// Constants
const BITS_PER_BYTE: u16 = 8;

/// Configuration for E2E Profile 5
#[derive(Debug, Clone)]
pub struct Profile5Config {
    /// Length of Data, in bits. The value shall be a multiple of 8.
    pub data_length: u16,
    /// An array of appropriately chosen Data IDs for protection against masquerading.
    pub data_id: u16,
    /// Maximum allowed delta between consecutive counters
    pub max_delta_counter: u8,
    /// Bit offset of E2E header in the Data[] array in bits.
    pub offset: u16,
}

/// Check Item for E2E Profile 5
#[derive(Debug, Clone)]
pub struct Profile5Check {
    rx_counter: u8,
    rx_crc: u16,
    calculated_crc: u16,
}

impl Default for Profile5Config {
    fn default() -> Self {
        Self {
            data_id: 0x1234,
            offset: 0x0000,
            data_length: 24, // 3bytes
            max_delta_counter: 1,
        }
    }
}

/// E2E Profile 5 Implementation
///
/// Implements AUTOSAR E2E Profile 4 protection mechanism
#[derive(Clone)]
pub struct Profile5 {
    config: Profile5Config,
    counter: u8,
    initialized: bool,
}

impl Profile5 {
    /// Validate configuration parameters
    fn validate_config(config: &Profile5Config) -> E2EResult<()> {
        validation::validate_min_data_length(
            config.data_length,
            3 * BITS_PER_BYTE,
            4096 * BITS_PER_BYTE,
        )?;
        validation::validate_offset_within_data(
            config.offset,
            config.data_length,
            3 * BITS_PER_BYTE,
        )?;
        validation::validate_counter_config_u8(config.max_delta_counter)?;
        Ok(())
    }
    /// Validate data length against min/max constraints
    fn validate_length(&self, len: u16) -> E2EResult<()> {
        let expected_bytes = self.config.data_length / BITS_PER_BYTE;
        validation::validate_data_length_exact(len, expected_bytes)
    }
    fn write_counter(&self, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::write_be_u8_at(data, offset + 2, self.counter);
    }
    fn compute_crc(&self, data: &[u8]) -> u16 {
        let crc: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
        let mut digest = crc.digest();
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        digest.update(&data[0..offset]); // crc calculation data before offset
        digest.update(&data[(offset + 2)..]); // crc calculation data after offset
        digest.update(&self.config.data_id.to_le_bytes());
        digest.finalize()
    }
    fn write_crc(&self, calculated_crc: u16, data: &mut [u8]) {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::write_le_u16_at(data, offset, calculated_crc);
    }
    fn increment_counter(&mut self) {
        self.counter = Counter8::increment_counter(self.counter);
    }
    fn read_counter(&self, data: &[u8]) -> u8 {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::read_be_u8_at(data, offset + 2)
    }
    fn read_crc(&self, data: &[u8]) -> u16 {
        let offset = field_ops::calculate_offset_bytes(self.config.offset);
        field_ops::read_le_u16_at(data, offset)
    }

    fn do_checks(&mut self, check_items: Profile5Check) -> E2EStatus {
        if check_items.calculated_crc != check_items.rx_crc {
            return E2EStatus::CrcError;
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

impl E2EProfile for Profile5 {
    type Config = Profile5Config;

    fn new(config: Self::Config) -> Self {
        // Validate config (panic if invalid in constructor for simplicity)
        Self::validate_config(&config).expect("Invalid Profile5 configuration");
        Self {
            config,
            counter: 0,
            initialized: false,
        }
    }

    fn protect(&mut self, data: &mut [u8]) -> E2EResult<()> {
        self.validate_length(data.len() as u16)?;
        self.write_counter(data);
        let calculated_crc = self.compute_crc(data);
        self.write_crc(calculated_crc, data);
        self.increment_counter();
        Ok(())
    }

    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus> {
        // Check data length
        self.validate_length(data.len() as u16)?;
        let check_items = Profile5Check {
            rx_counter: self.read_counter(data),
            rx_crc: self.read_crc(data),
            calculated_crc: self.compute_crc(data),
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
    fn test_profile5_basic_example() {
        let config = Profile5Config {
            data_length: 8 * BITS_PER_BYTE,
            ..Default::default()
        };

        let mut profile_tx = Profile5::new(config.clone());
        let mut profile_rx = Profile5::new(config);

        let mut data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        profile_tx.protect(&mut data).unwrap();
        // crc check
        assert_eq!(data[0], 0x1c);
        assert_eq!(data[1], 0xca);
        // counter check
        assert_eq!(data[2], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
    #[test]
    fn test_profile5_offset_example() {
        let config = Profile5Config {
            offset: 8 * BITS_PER_BYTE,
            data_length: 16 * BITS_PER_BYTE,
            ..Default::default()
        };

        let mut profile_tx = Profile5::new(config.clone());
        let mut profile_rx = Profile5::new(config);

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // crc check
        assert_eq!(data[8], 0x28);
        assert_eq!(data[9], 0x91);
        // counter check
        assert_eq!(data[10], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
    #[test]
    fn test_profile5_counter_wraparound() {
        let config = Profile5Config {
            offset: 8 * BITS_PER_BYTE,
            data_length: 16 * BITS_PER_BYTE,
            ..Default::default()
        };

        let mut profile_tx = Profile5::new(config.clone());
        let mut profile_rx = Profile5::new(config);

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // crc check
        assert_eq!(data[8], 0x28);
        assert_eq!(data[9], 0x91);
        // counter check
        assert_eq!(data[10], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        for i in 1u8..=0xFF {
            profile_tx.protect(&mut data).unwrap();
            // counter check
            assert_eq!(data[10], i);
            assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        }
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[10], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
}
