//! # E2E Profile 22 Implementation
//!
//! Profile 22 is designed for protecting small data packets
//! with low overhead. It uses:
//! - 8-bit CRC for data integrity
//! - 4-bit counter for sequence checking (0-14)
//! - 16-bit Data ID for masquerade prevention
//!
//! # Data layout
//! [DATA ... | CRC(1B) | HDR(1B) | DATA ...]
//! - HDR (bits 3..0) : counter

use crate::{E2EError, E2EProfile, E2EResult, E2EStatus};
use crc::{Crc, CRC_8_AUTOSAR};

// Constants
const COUNTER_MASK: u8 = 0x0F;
const COUNTER_MAX: u8 = 15;
const COUNTER_MODULO: u8 = 16;
const BITS_PER_BYTE: usize = 8;
const HEADER_LENGTH_BYTES: usize = 2;
const DATA_ID_NUMBER: usize = 16;

/// Configuration for E2E Profile 22
#[derive(Debug, Clone)]
pub struct Profile22Config {
    /// Length of Data, in bits. The value shall be a multiple of 8.
    pub data_length: usize,
    /// An array of appropriately chosen Data IDs for protection against masquerading.
    pub data_id_list: [u8; DATA_ID_NUMBER],
    /// Maximum allowed delta between consecutive counters
    pub max_delta_counter: u8,
    /// Bit offset of E2E header in the Data[] array in bits.
    pub offset: usize,
}

impl Default for Profile22Config {
    fn default() -> Self {
        Self {
            data_length: 64,        // bits
            data_id_list: [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x10],
            max_delta_counter: 1,
            offset : 0,             // bits
        }
    }
}

/// Check Item for E2E Profile 4
#[derive(Debug, Clone)]
pub struct Profile22Check {
    rx_counter: u8,
    rx_crc: u8,
    calculated_crc: u8,
}
/// E2E Profile 22 Implementation
///
/// Implements AUTOSAR E2E Profile 22 protection mechanism
#[derive(Clone)]
pub struct Profile22 {
    config: Profile22Config,
    counter: u8,
}

impl Profile22 {
    /// Validate configuration parameters
    fn validate_config(config: &Profile22Config) -> E2EResult<()> {
        if (config.data_length % BITS_PER_BYTE) != 0 {
            return Err(E2EError::InvalidConfiguration(
                "Data length shall be a multiple of 8".into()
            ));
        }

        if config.max_delta_counter == 0 || config.max_delta_counter > COUNTER_MAX  {
            return Err(E2EError::InvalidConfiguration(
                format!("Max delta counter must be between 1 and {}", COUNTER_MAX )
            ));
        }

        Ok(())
    }
    /// Validate data length against min/max constraints
    fn validate_length(&self, len: usize) -> E2EResult<()> {
        let expected_bytes = self.config.data_length / BITS_PER_BYTE;
        if len != expected_bytes {
            return Err(E2EError::InvalidDataFormat(format!(
                "Expected {} bytes, got {} bytes",
                expected_bytes, len
            )));
        }
        let expected_bytes = self.config.offset.div_ceil(BITS_PER_BYTE) + HEADER_LENGTH_BYTES;
        if len < expected_bytes  {
            return Err(E2EError::InvalidDataFormat(format!(
                "Data Length shall be equal to or larger than offset + {} : Expected {} bytes, got {} bytes",
                HEADER_LENGTH_BYTES, expected_bytes, len
            )));
        }
        Ok(())
    }
    fn increment_counter(&mut self) {
        self.counter = (self.counter + 1) % COUNTER_MODULO;
    }
    fn write_counter(&self, data: &mut[u8]) {
        let byte_idx = self.config.offset >> 3;

        data[byte_idx+1] = (data[byte_idx+1] & 0xF0) | self.counter;        
    }
    fn read_counter(&self, data: &[u8]) -> u8{
        let byte_idx = self.config.offset >> 3;

        data[byte_idx+1] & COUNTER_MASK        
    }
    fn write_crc(&self, calculated_crc: u8, data: &mut[u8]) {
        let byte_position = self.config.offset / BITS_PER_BYTE;
        data[byte_position] = calculated_crc;
    }
    fn read_crc(&self, data: &[u8]) -> u8 {
        let byte_position = self.config.offset / BITS_PER_BYTE;
        data[byte_position]
    }
    fn compute_crc(&self, data: &[u8]) -> u8 {
        let crc: Crc<u8> = Crc::<u8>::new(&CRC_8_AUTOSAR);
        let mut digest = crc.digest();
        let offset_byte = self.config.offset / BITS_PER_BYTE;
        digest.update(&data[0..offset_byte]); // crc calculation data before offset
        digest.update(&data[(offset_byte+1)..]); // crc calculation data after offset
        digest.update(&[self.config.data_id_list[self.read_counter(data) as usize]]); // crc calculation data id
        digest.finalize()
    }
    fn do_checks(&mut self, check_items : Profile22Check) -> E2EStatus {
        if check_items.calculated_crc != check_items.rx_crc {
            return E2EStatus::CrcError
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
            (COUNTER_MODULO + received_counter - self.counter) % COUNTER_MODULO
        }
    }
    fn validate_counter(&self, rx_counter: u8) -> E2EStatus {
        let delta = self.check_counter_delta(rx_counter);

        if delta == 0 {
            E2EStatus::Repeated
        } else if delta == 1 {
            E2EStatus::Ok
        } else if delta >= 2 && delta <= self.config.max_delta_counter {
            E2EStatus::OkSomeLost
        } else {
            E2EStatus::WrongSequence
        }
    }
}

impl E2EProfile for Profile22 {
    type Config = Profile22Config;

    fn new(config: Self::Config) -> Self {
        // Validate config (panic if invalid in constructor for simplicity)
        Self::validate_config(&config).expect("Invalid Profile22 configuration");
        Self {
            config,
            counter: 0,
        }
    }

    fn protect(&mut self, data: &mut [u8]) -> E2EResult<()> {
        self.validate_length(data.len())?;
        self.increment_counter();
        self.write_counter(data);
        let calculated_crc = self.compute_crc(data);
        self.write_crc(calculated_crc, data);
        Ok(())
    }

    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus> {
        // Check data length
        self.validate_length(data.len())?;
        let check_items = Profile22Check{rx_counter: self.read_counter(data), 
                                                        rx_crc: self.read_crc(data), 
                                                        calculated_crc: self.compute_crc(data)};
        let status = self.do_checks(check_items);
        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_profile22_basic_example() {
        let mut profile_tx = Profile22::new(Profile22Config::default());
        let mut profile_rx = Profile22::new(Profile22Config::default());

        let mut data = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x1b);
        assert_eq!(data[1], 0x01);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x98);
        assert_eq!(data[1], 0x02);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x31);
        assert_eq!(data[1], 0x03);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x0d);
        assert_eq!(data[1], 0x04);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x18);
        assert_eq!(data[1], 0x05);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x9b);
        assert_eq!(data[1], 0x06);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x65);
        assert_eq!(data[1], 0x07);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x08);
        assert_eq!(data[1], 0x08);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x1d);
        assert_eq!(data[1], 0x09);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x9e);
        assert_eq!(data[1], 0x0a);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x37);
        assert_eq!(data[1], 0x0b);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x0b);
        assert_eq!(data[1], 0x0c);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x1e);
        assert_eq!(data[1], 0x0d);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x9d);
        assert_eq!(data[1], 0x0e);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0xcd);
        assert_eq!(data[1], 0x0f);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        profile_tx.protect(&mut data).unwrap();
        assert_eq!(data[0], 0x0e);
        assert_eq!(data[1], 0x00);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
    }

    #[test]
    fn test_profile22_offset_example() {
        let config = Profile22Config {
            offset : 64,
            data_length : 128,
            ..Default::default()
        };

        let mut profile_tx = Profile22::new(config.clone());
        let mut profile_rx = Profile22::new(config);

        let mut data1 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        profile_tx.protect(&mut data1).unwrap();
        assert_eq!(data1[8], 0x14);
        assert_eq!(data1[9], 0x01);
        assert_eq!(profile_rx.check(&data1).unwrap(), E2EStatus::Ok);
    }
}