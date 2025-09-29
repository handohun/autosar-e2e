//! # E2E Profile 7 Implementation
//!
//! Profile 7 is designed for protecting large data packets
//! with low overhead. It uses:
//! - 64-bit CRC for data integrity
//! - 32-bit counter for sequence checking
//! - 32-bit Data ID for masquerade prevention
//! - 32-bit Data Length to support dynamic size data
//!
//! # Data layout
//! [DATA ... | CRC(8B) | LENGTH(4B) | COUNTER(4B) | ID (4B) | DATA ...]
use crate::{E2EError, E2EProfile, E2EResult, E2EStatus};
use crc::{Crc, CRC_64_XZ};

// Constants
const BITS_PER_BYTE: u32 = 8;
const COUNTER_MAX: u32 = 0xFFFFFFFF;
const COUNTER_MODULO: u64 = 0x100000000;

/// Configuration for E2E Profile 7
#[derive(Debug, Clone)]
pub struct Profile7Config {
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

/// Check Item for E2E Profile 7
#[derive(Debug, Clone)]
pub struct Profile7Check {
    rx_data_length: u32,
    rx_counter: u32,
    rx_data_id: u32,
    rx_crc: u64,
    calculated_crc: u64,
    data_len: u32,
}

impl Default for Profile7Config {
    fn default() -> Self {
        Self {
            data_id: 0x0a0b0c0d,
            offset: 0x00000000,
            min_data_length: 160,   // 20bytes
            max_data_length: 32768, // 4096bytes
            max_delta_counter: 1,
        }
    }
}

/// E2E Profile 7 Implementation
///
/// Implements AUTOSAR E2E Profile 7 protection mechanism
#[derive(Clone)]
pub struct Profile7 {
    config: Profile7Config,
    counter: u32,
    initialized: bool,
}

impl Profile7 {
    /// Validate configuration parameters
    fn validate_config(config: &Profile7Config) -> E2EResult<()> {
        if config.min_data_length < 20 * BITS_PER_BYTE {
            return Err(E2EError::InvalidConfiguration(
                "Minimum Data length shall be larger than 20B".into(),
            ));
        }
        if config.max_data_length < config.min_data_length {
            return Err(E2EError::InvalidConfiguration(
                "Maximum Data length shall be larger than MinDataLength".into(),
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
    fn validate_length(&self, len: u32) -> E2EResult<()> {
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
    fn write_data_length(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        let len32 = data.len() as u32;
        data[offset + 8..=offset + 11].copy_from_slice(&len32.to_be_bytes());
    }
    fn write_counter(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 12..=offset + 15].copy_from_slice(&self.counter.to_be_bytes());
    }
    fn write_data_id(&self, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset + 16..=offset + 19].copy_from_slice(&self.config.data_id.to_be_bytes());
    }
    fn compute_crc(&self, data: &[u8]) -> u64 {
        let crc: Crc<u64> = Crc::<u64>::new(&CRC_64_XZ);
        let mut digest = crc.digest();
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        digest.update(&data[0..offset]); // crc calculation data before offset
        digest.update(&data[(offset + 8)..]); // crc calculation data after offset
        digest.finalize()
    }
    fn write_crc(&self, calculated_crc: u64, data: &mut [u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset..=offset + 7].copy_from_slice(&calculated_crc.to_be_bytes());
    }
    fn increment_counter(&mut self) {
        self.counter = if self.counter == COUNTER_MAX {
            0x00000000
        } else {
            (self.counter + 1) & COUNTER_MAX
        };
    }

    fn read_data_length(&self, data: &[u8]) -> u32 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([
            data[offset + 8],
            data[offset + 9],
            data[offset + 10],
            data[offset + 11],
        ])
    }
    fn read_counter(&self, data: &[u8]) -> u32 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([
            data[offset + 12],
            data[offset + 13],
            data[offset + 14],
            data[offset + 15],
        ])
    }
    fn read_data_id(&self, data: &[u8]) -> u32 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([
            data[offset + 16],
            data[offset + 17],
            data[offset + 18],
            data[offset + 19],
        ])
    }
    fn read_crc(&self, data: &[u8]) -> u64 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u64::from_be_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
            data[offset + 4],
            data[offset + 5],
            data[offset + 6],
            data[offset + 7],
        ])
    }

    fn do_checks(&mut self, check_items: Profile7Check) -> E2EStatus {
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
    fn check_counter_delta(&self, received_counter: u32) -> u32 {
        if received_counter >= self.counter {
            received_counter - self.counter
        } else {
            // Handle wrap-around
            ((COUNTER_MODULO + received_counter as u64 - self.counter as u64) % COUNTER_MODULO)
                as u32
        }
    }
    fn validate_counter(&self, rx_counter: u32) -> E2EStatus {
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

impl E2EProfile for Profile7 {
    type Config = Profile7Config;

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
        let check_items = Profile7Check {
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
    fn test_profile7_basic_example() {
        let mut profile_tx = Profile7::new(Profile7Config::default()).unwrap();
        let mut profile_rx = Profile7::new(Profile7Config::default()).unwrap();

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // CRC check
        assert_eq!(data[0], 0x1f);
        assert_eq!(data[1], 0xb2);
        assert_eq!(data[2], 0xe7);
        assert_eq!(data[3], 0x37);
        assert_eq!(data[4], 0xfc);
        assert_eq!(data[5], 0xed);
        assert_eq!(data[6], 0xbc);
        assert_eq!(data[7], 0xd9);
        // length check
        assert_eq!(data[8], 0x00);
        assert_eq!(data[9], 0x00);
        assert_eq!(data[10], 0x00);
        assert_eq!(data[11], 0x18);
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
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }

    #[test]
    fn test_profile7_offset_example() {
        let config = Profile7Config {
            offset: 64,
            ..Default::default()
        };

        let mut profile_tx = Profile7::new(config.clone()).unwrap();
        let mut profile_rx = Profile7::new(config).unwrap();

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // CRC check
        assert_eq!(data[8], 0x17);
        assert_eq!(data[9], 0xf7);
        assert_eq!(data[10], 0xc8);
        assert_eq!(data[11], 0x17);
        assert_eq!(data[12], 0x32);
        assert_eq!(data[13], 0x38);
        assert_eq!(data[14], 0x65);
        assert_eq!(data[15], 0xa8);
        // length check
        assert_eq!(data[16], 0x00);
        assert_eq!(data[17], 0x00);
        assert_eq!(data[18], 0x00);
        assert_eq!(data[19], 0x20);
        // counter check
        assert_eq!(data[20], 0x00);
        assert_eq!(data[21], 0x00);
        assert_eq!(data[22], 0x00);
        assert_eq!(data[23], 0x00);
        // data id check
        assert_eq!(data[24], 0x0a);
        assert_eq!(data[25], 0x0b);
        assert_eq!(data[26], 0x0c);
        assert_eq!(data[27], 0x0d);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
    #[test]
    fn test_profile7_counter_wraparound() {
        let config = Profile7Config {
            offset: 64,
            ..Default::default()
        };

        let mut profile_tx = Profile7::new(config.clone()).unwrap();
        let mut profile_rx = Profile7::new(config).unwrap();

        let mut data = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00,
        ];
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[20], 0x00);
        assert_eq!(data[21], 0x00);
        assert_eq!(data[22], 0x00);
        assert_eq!(data[23], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[20], 0x00);
        assert_eq!(data[21], 0x00);
        assert_eq!(data[22], 0x00);
        assert_eq!(data[23], 0x01);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_rx.counter = 0xFFFFFFFE;
        profile_tx.counter = 0xFFFFFFFF;
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[20], 0xFF);
        assert_eq!(data[21], 0xFF);
        assert_eq!(data[22], 0xFF);
        assert_eq!(data[23], 0xFF);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        // counter check
        assert_eq!(data[20], 0x00);
        assert_eq!(data[21], 0x00);
        assert_eq!(data[22], 0x00);
        assert_eq!(data[23], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }
}
