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
use crate::{E2EError, E2EProfile, E2EResult, E2EStatus};
use crc::{Crc, CRC_16_IBM_3740};

// Constants
const COUNTER_MAX: u8 = 0xFF;
const BITS_PER_BYTE: u16 = 8;
const COUNTER_MODULO: u16 = 0x100;

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
        if config.data_length < 3 * BITS_PER_BYTE || 4096 * BITS_PER_BYTE < config.data_length {
            return Err(E2EError::InvalidConfiguration(
                "Minimum Data length shall be between 3B and 4096B".into(),
            ));
        }
        if config.data_length - 3 * BITS_PER_BYTE < config.offset {
            return Err(E2EError::InvalidConfiguration(
                "Offset shall be between 0 and data length - 3B".into(),
            ));
        }
        if config.max_delta_counter == 0 || config.max_delta_counter == COUNTER_MAX {
            return Err(E2EError::InvalidConfiguration(format!(
                "Max delta counter must be between 1 and {}",
                COUNTER_MAX
            )));
        }
        Ok(())
    }
    /// Validate data length against min/max constraints
    fn validate_length(&self, len: u16) -> E2EResult<()> {
        let expected_bytes = self.config.data_length / BITS_PER_BYTE;
        if len != expected_bytes {
            return Err(E2EError::InvalidDataFormat(format!(
                "Expected {} bytes, got {} bytes",
                expected_bytes, len
            )));
        }
        Ok(())
    }
    fn write_counter(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 2] = self.counter;
    }
    fn compute_crc(&self, data: &[u8]) -> u16 {
        let crc: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
        let mut digest = crc.digest();
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        digest.update(&data[0..offset]); // crc calculation data before offset
        digest.update(&data[(offset + 2)..]); // crc calculation data after offset
        digest.update(&self.config.data_id.to_le_bytes());
        digest.finalize()
    }
    fn write_crc(&self, calculated_crc: u16, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset..=offset + 1].copy_from_slice(&calculated_crc.to_le_bytes());
    }
    fn increment_counter(&mut self) {
        self.counter = (self.counter as u16 + 1) as u8 & COUNTER_MAX;
    }
    fn read_counter(&self, data: &[u8]) -> u8 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 2]
    }
    fn read_crc(&self, data: &[u8]) -> u16 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u16::from_le_bytes([data[offset], data[offset + 1]])
    }

    fn do_checks(&mut self, check_items: Profile5Check) -> E2EStatus {
        if check_items.calculated_crc != check_items.rx_crc {
            return E2EStatus::CrcError;
        }
        let status = self.validate_counter(check_items.rx_counter);
        self.counter = check_items.rx_counter;
        status
    }
    /// Check if counter delta is within acceptable range
    fn check_counter_delta(&self, received_counter: u8) -> u8 {
        if received_counter >= self.counter {
            received_counter - self.counter
        } else {
            // Handle wrap-around
            ((COUNTER_MODULO + received_counter as u16 - self.counter as u16) % COUNTER_MODULO)
                as u8
        }
    }
    fn validate_counter(&self, rx_counter: u8) -> E2EStatus {
        let delta = self.check_counter_delta(rx_counter);

        if delta == 0 {
            if self.initialized {
                E2EStatus::Repeated
            } else {
                E2EStatus::Ok
            }
        } else if delta == 1 {
            E2EStatus::Ok
        } else if delta >= 2 && delta <= self.config.max_delta_counter {
            E2EStatus::OkSomeLost
        } else {
            E2EStatus::WrongSequence
        }
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
