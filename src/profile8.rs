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
use crate::{E2EError, E2EProfile, E2EResult, E2EStatus};
use crc::{Crc, CRC_32_AUTOSAR};

// Constants
const BITS_PER_BYTE : u32 = 8;
const COUNTER_MAX : u32 = 0xFFFFFFFF;
const COUNTER_MODULO : u64 = 0x100000000;

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
    data_len : u32,
}

impl Default for Profile8Config {
    fn default() -> Self {
        Self {
            data_id : 0x0a0b0c0d,
            offset : 0x00000000,
            min_data_length: 128, // 16bytes
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
        if config.min_data_length < 16*BITS_PER_BYTE || 4294967295 < config.min_data_length{
            return Err(E2EError::InvalidConfiguration(
                "Minimum Data length shall be between 16B and 536870911B".into()
            ));
        }
        if config.max_data_length < config.min_data_length || 4294967295 < config.max_data_length{
            return Err(E2EError::InvalidConfiguration(
                "Minimum Data length shall be between MinDataLength and 536870911B".into()
            ));
        }
        if config.max_delta_counter == 0 || config.max_delta_counter == COUNTER_MAX  {
            return Err(E2EError::InvalidConfiguration(
                format!("Max delta counter must be between 1 and {}", COUNTER_MAX )
            ));
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
    fn write_data_length(&self, data: &mut[u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        let len32 = data.len() as u32;
        data[offset+4..=offset+7].copy_from_slice(&len32.to_be_bytes());
    }
    fn write_counter(&self, data: &mut[u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset+8..=offset+11].copy_from_slice(&self.counter.to_be_bytes());
    }
    fn write_data_id(&self, data: &mut[u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset+12..=offset+15].copy_from_slice(&self.config.data_id.to_be_bytes());
    }
    fn compute_crc(&self, data: &[u8]) -> u32 {
        let crc: Crc<u32> = Crc::<u32>::new(&CRC_32_AUTOSAR);
        let mut digest = crc.digest();
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        digest.update(&data[0..offset]); // crc calculation data before offset
        digest.update(&data[(offset+4)..]); // crc calculation data after offset
        digest.finalize()
    }
    fn write_crc(&self, calculated_crc: u32, data: &mut[u8]) {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        data[offset..=offset+3].copy_from_slice(&calculated_crc.to_be_bytes());

    }
    fn increment_counter(&mut self) {
        self.counter = if self.counter == COUNTER_MAX {0x0000} else { (self.counter + 1) & COUNTER_MAX};
    }

    fn read_data_length(&self, data: &[u8]) -> u32 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([data[offset+4], data[offset + 5], data[offset + 6], data[offset + 7]])
    }
    fn read_counter(&self, data: &[u8]) -> u32{
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([data[offset + 8], data[offset + 9], data[offset + 10], data[offset + 11]])
    }
    fn read_data_id(&self, data: &[u8]) -> u32 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([data[offset + 12], data[offset + 13], data[offset + 14], data[offset + 15]])
    }
    fn read_crc(&self, data: &[u8]) -> u32 {
        let offset = (self.config.offset / BITS_PER_BYTE) as usize;
        u32::from_be_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
    }

    fn do_checks(&mut self, check_items : Profile8Check) -> E2EStatus {
        if check_items.calculated_crc != check_items.rx_crc {
            return E2EStatus::CrcError
        }
        if check_items.rx_data_id != self.config.data_id {
            return E2EStatus::DataIdError
        }
        if check_items.rx_data_length != check_items.data_len {
            return E2EStatus::DataLengthError
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
            ((COUNTER_MODULO + received_counter as u64 - self.counter as u64) % COUNTER_MODULO) as u32
        }
    }
    fn validate_counter(&self, rx_counter: u32) -> E2EStatus {
        let delta = self.check_counter_delta(rx_counter);

        if delta == 0 {
            if self.initialized {
                E2EStatus::Repeated
            }
            else {
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
        let check_items = Profile8Check{rx_data_length: self.read_data_length(data), 
                                                        rx_counter: self.read_counter(data),
                                                        rx_crc: self.read_crc(data),
                                                        rx_data_id: self.read_data_id(data),
                                                        calculated_crc: self.compute_crc(data),
                                                        data_len : data.len() as u32};
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

        let mut data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00];
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
            offset : 64,
            ..Default::default()
        };

        let mut profile_tx = Profile8::new(config.clone());
        let mut profile_rx = Profile8::new(config);

        let mut data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00];
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
            offset : 64,
            ..Default::default()
        };

        let mut profile_tx = Profile8::new(config.clone());
        let mut profile_rx = Profile8::new(config);

        let mut data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
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