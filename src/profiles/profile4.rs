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
use crate::{E2EError, E2EProfile, E2EResult, E2EStatus};
use crc::{Crc, CRC_32_AUTOSAR};

// Constants
const BITS_PER_BYTE: u16 = 8;
const COUNTER_MAX: u16 = 0xFFFF;
const COUNTER_MODULO: u32 = 0x10000;

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
        if config.min_data_length < 12 * BITS_PER_BYTE
            || 4096 * BITS_PER_BYTE < config.min_data_length
        {
            return Err(E2EError::InvalidConfiguration(
                "Minimum Data length shall be between 12B and 4096B".into(),
            ));
        }
        if config.max_data_length < config.min_data_length || 4096 * 8 < config.max_data_length {
            return Err(E2EError::InvalidConfiguration(
                "Maximum Data length shall be between MinDataLength and 4096B".into(),
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
        let min_bytes = self.config.min_data_length / BITS_PER_BYTE;
        let max_bytes = self.config.max_data_length / BITS_PER_BYTE;
        if len < min_bytes || max_bytes < len {
            return Err(E2EError::InvalidDataFormat(format!(
                "Expected {} - {} bytes, got {} bytes",
                min_bytes, max_bytes, len
            )));
        }
        Ok(())
    }
    fn increment_counter(&mut self) {
        self.counter = (self.counter as u32 + 1) as u16 & COUNTER_MAX;
    }
    fn write_data_length(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        let len16 = data.len() as u16;
        data[offset..=offset + 1].copy_from_slice(&len16.to_be_bytes());
    }
    fn write_counter(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 2..=offset + 3].copy_from_slice(&self.counter.to_be_bytes());
    }
    fn write_data_id(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 4..=offset + 7].copy_from_slice(&self.config.data_id.to_be_bytes());
    }
    fn compute_crc(&self, data: &[u8]) -> u32 {
        let crc: Crc<u32> = Crc::<u32>::new(&CRC_32_AUTOSAR);
        let mut digest = crc.digest();
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        digest.update(&data[0..offset + 8]); // crc calculation data before offset
        digest.update(&data[(offset + 12)..]); // crc calculation data after offset
        digest.finalize()
    }
    fn write_crc(&self, calculated_crc: u32, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 8..=offset + 11].copy_from_slice(&calculated_crc.to_be_bytes());
    }
    fn read_data_length(&self, data: &[u8]) -> u16 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u16::from_be_bytes([data[offset], data[offset + 1]])
    }
    fn read_counter(&self, data: &[u8]) -> u16 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u16::from_be_bytes([data[offset + 2], data[offset + 3]])
    }
    fn read_data_id(&self, data: &[u8]) -> u32 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ])
    }
    fn read_crc(&self, data: &[u8]) -> u32 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([
            data[offset + 8],
            data[offset + 9],
            data[offset + 10],
            data[offset + 11],
        ])
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
    /// Check if counter delta is within acceptable range
    fn check_counter_delta(&self, received_counter: u16) -> u16 {
        if received_counter >= self.counter {
            received_counter - self.counter
        } else {
            // Handle wrap-around
            ((COUNTER_MODULO + received_counter as u32 - self.counter as u32) % COUNTER_MODULO)
                as u16
        }
    }
    fn validate_counter(&self, rx_counter: u16) -> E2EStatus {
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

impl E2EProfile for Profile4 {
    type Config = Profile4Config;

    fn new(config: Self::Config) -> E2EResult<Self> {
        // Validate config
        Self::validate_config(&config)?;
        Ok(Self {
            config,
            counter: 0,
            initialized: false,
        })
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
        let mut profile_tx = Profile4::new(Profile4Config::default()).unwrap();
        let mut profile_rx = Profile4::new(Profile4Config::default()).unwrap();

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

        let mut profile_tx = Profile4::new(config.clone()).unwrap();
        let mut profile_rx = Profile4::new(config).unwrap();

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

        let mut profile_tx = Profile4::new(config.clone()).unwrap();
        let mut profile_rx = Profile4::new(config).unwrap();

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
