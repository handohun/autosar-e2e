//! # E2E Profile 11 Implementation
//!
//! Profile 11 is designed for protecting small data packets (up to DEFAULT_MAX_DATA_LENGTH bytes)
//! with low overhead. It uses:
//! - 8-bit CRC for data integrity
//! - 4-bit counter for sequence checking
//! - 4-bit Data ID nibble for addressing verification
//!
//! # Data layout
//! [CRC(1B) | HDR(1B) | DATA ...]
//! - HDR (bits 7..4) : DI_hi_nibble(nibble mode) OR data(both mode)
//! - HDR (bits 3..0) : counter
//! 
//! # Modes
//!
//! Profile 11 supports two main modes:
//! - **Both(11A)**: full 16-bit Data-ID is implicit (only used in CRC).
//! - **Nibble(11C)**: high 4-bit is explicit in the header (1..=0xE recommended),
//!   low 8-bit is implicit (in CRC).


// INCLUDE
use crate::{E2EError, E2EProfile, E2EResult, E2EStatus};
use crc::{Crc, Algorithm};

// CONSTANT
const COUNTER_MASK: u8 = 0x0F;
const UPPER_NIBBLE_MASK: u8 = 0xF0;
const LOWER_NIBBLE_MASK: u8 = 0x0F;
const MAX_COUNTER_VALUE: u8 = 0x0E;
const MIN_DATA_LENGTH: usize = 2;
const DEFAULT_MAX_DATA_LENGTH: usize = 240;
// profile 11 use CRC_8_SAE_J1850 but with different init and xorout value, so algo is defined
const CRC8_ALGO: Algorithm<u8> = Algorithm { width: 8, poly: 0x1d, init: 0x00, refin: false, refout: false, xorout: 0x00, check: 0x4b, residue: 0xc4 };

/// Data-ID mode for Profile 11.
/// 
/// # Variants
///
/// * `Both(u16)` - Profile 11A: The complete 16-bit Data-ID is only used 
///   implicitly for CRC calculation. The header preserves the original 
///   upper nibble of the data.
///
/// * `Nibble(u16)` - Profile 11C: The upper 4 bits of the Data-ID are 
///   stored explicitly in the header, while the lower 8 bits are used 
///   implicitly for CRC calculation. Recommended range: 0x100-0xE00.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile11IdMode {
    Both(u16),
    Nibble (u16), // Only lower 12 bits used: 0x000..=0xFFF
}

/// Configuration for E2E Profile 11
#[derive(Debug, Clone)]
pub struct Profile11Config {
    /// Profile mode (11A or 11C)
    pub mode: Profile11IdMode,
    /// Maximum allowed delta between consecutive counters
    pub max_delta_counter: u8,
    /// Maximum data length (up to DEFAULT_MAX_DATA_LENGTH bytes)
    pub max_data_length: usize,
}

impl Default for Profile11Config {
    fn default() -> Self {
        Self {
            mode: Profile11IdMode::Nibble(0x100),
            max_delta_counter: 1,
            max_data_length: DEFAULT_MAX_DATA_LENGTH,
        }
    }
}

/// E2E Profile 11 Implementation
///
/// Implements AUTOSAR E2E Profile 11 protection mechanism with support
/// for both 11A and 11C variants.
#[derive(Clone)]
pub struct Profile11 {
    config: Profile11Config,
    counter: u8,
}

impl Profile11 {
    /// Validate configuration parameters
    fn validate_config(config: &Profile11Config) -> E2EResult<()> {
        if config.max_data_length > DEFAULT_MAX_DATA_LENGTH {
            return Err(E2EError::InvalidConfiguration(
                format!("Maximum data length for Profile 11 is {} bytes", DEFAULT_MAX_DATA_LENGTH)
            ));
        }

        if config.max_delta_counter == 0 || config.max_delta_counter > MAX_COUNTER_VALUE {
            return Err(E2EError::InvalidConfiguration(
                format!("Max delta counter must be between 1 and {}", MAX_COUNTER_VALUE)
            ));
        }

        Ok(())
    }
    /// Validate data length against min/max constraints
    fn validate_length(&self, len: usize) -> E2EResult<()> {
        if len < MIN_DATA_LENGTH {
            return Err(E2EError::InvalidDataFormat(format!(
                "Data length {} is below minimum required length of {}",
                len, MIN_DATA_LENGTH
            )));
        }
        if len > self.config.max_data_length {
            return Err(E2EError::InvalidDataFormat(format!(
                "Data length {} exceeds configured maximum of {}",
                len, self.config.max_data_length
            )));
        }
        Ok(())
    }
    /// Update Crc with ID
    fn update_crc_with_id(&self, digest: &mut crc::Digest<u8>) {
        match self.config.mode {
            Profile11IdMode::Both(id) => {
                digest.update(&id.to_le_bytes());
            }
            Profile11IdMode::Nibble(id) => {
                digest.update(&[id.to_le_bytes()[0], 0x00]);
            }
        }
    }
    fn compose_header(&self, data: &[u8]) -> u8 {
        let nibble = match self.config.mode {
            Profile11IdMode::Both(_) => data[1] & UPPER_NIBBLE_MASK, // keep high nibble
            Profile11IdMode::Nibble(id) => {
                (id.to_le_bytes()[1] & LOWER_NIBBLE_MASK) << 4 // low nibble of high byte
            }
        };
        nibble | (self.counter & LOWER_NIBBLE_MASK)
    }
    fn calculate_crc(&self, data: &[u8]) -> u8 {
        let crc: Crc<u8> = Crc::<u8>::new(&CRC8_ALGO);
        let mut digest = crc.digest();
        self.update_crc_with_id(&mut digest);
        digest.update(&data[1..]);
        digest.finalize()
    }
    fn validate_id(&self, data: &[u8]) -> E2EStatus {
        if let Profile11IdMode::Nibble(id) = self.config.mode {
            let id_bytes = id.to_le_bytes();
            let expected_nibble = id_bytes[1] & LOWER_NIBBLE_MASK;
            let got_nibble = (data[1] & UPPER_NIBBLE_MASK) >> 4;
            if got_nibble != expected_nibble {
                return E2EStatus::DataIdError;
            }
        }
        E2EStatus::Ok // Always Ok for Both mode
    }
    /// Check if counter delta is within acceptable range
    fn check_counter_delta(&self, received_counter: u8) -> u8 {
        if received_counter >= self.counter {
            received_counter - self.counter
        } else {
            // Handle wrap-around
            (COUNTER_MASK + received_counter - self.counter) % COUNTER_MASK
        }
    }
    fn validate_counter(&mut self, rx_counter: u8) -> E2EStatus {
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

impl E2EProfile for Profile11 {
    type Config = Profile11Config;

    fn new(config: Self::Config) -> Self {
        // Validate config (panic if invalid in constructor for simplicity)
        Self::validate_config(&config).expect("Invalid Profile11 configuration");
        Self {
            config,
            counter: 0,
        }
    }

    fn protect(&mut self, data: &mut [u8]) -> E2EResult<()> {
        // Check data length
        self.validate_length(data.len())?;

        // Increment counter
        self.counter = (self.counter + 1) % COUNTER_MASK;

        //Update header
        data[1] = self.compose_header(data);

        // Calculate and set CRC in byte 0
        data[0] = self.calculate_crc(data);
        
        Ok(())
    }

    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus> {
        // Check data length
        self.validate_length(data.len())?;

        // Check Crc
        if self.calculate_crc(data) != data[0] {
            return Ok(E2EStatus::CrcError);
        }
        
        // Check Data ID (for nibble mode)
        match self.validate_id(data) {
            E2EStatus::Ok => {}
            other => return Ok(other),
        }
        
        // Check counter
        let rx_counter = data[1] & COUNTER_MASK;
        let status = self.validate_counter(rx_counter);

        // Update counter
        self.counter = rx_counter;

        Ok(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profile11_new() {
        let config = Profile11Config::default();
        let profile = Profile11::new(config);
        assert_eq!(profile.counter, 0);
    }

    #[test]
    fn test_profile11_protect_variant_11c() {
        let config = Profile11Config {
            mode: Profile11IdMode::Nibble(0x1A20),
            ..Default::default()
        };
        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);

        // Create test data
        let mut data = vec![0x00, 0x00, 0x12, 0x34, 0x56];
        
        // Protect the data
        profile_tx.protect(&mut data).unwrap();
        
        // Check that header was modified
        assert_eq!(data[0], 0x54); // CRC should be calculated
        assert_eq!(data[1] & LOWER_NIBBLE_MASK, 1); // Counter should be 1
        assert_eq!(data[1] >> 4, 0xA); // DataIDNibble should be 0xA
        assert_eq!(&data[2..], &[0x12, 0x34, 0x56]); // User data should remain unchanged

        let status = profile_rx.check(&data).unwrap();
        assert_eq!(status, E2EStatus::Ok);

    }

    #[test]
    fn test_profile11_protect_variant_11a() {
        let config = Profile11Config {
            mode: Profile11IdMode::Both(0x1234),
            ..Default::default()
        };
        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);
        
        // Create test data with upper nibble set
        let mut data = vec![0x00, 0xF0, 0x12, 0x34];
        
        // Protect the data
        profile_tx.protect(&mut data).unwrap();
        
        // Check that header was modified correctly
        assert_eq!(data[0], 0x9D); // CRC should be calculated
        assert_eq!(data[1] & LOWER_NIBBLE_MASK, 1); // Counter should be 1
        assert_eq!(data[1] & UPPER_NIBBLE_MASK, 0xF0); // Upper nibble should be preserved
        assert_eq!(&data[2..], &[0x12, 0x34]); // User data should remain unchanged

        let status = profile_rx.check(&data).unwrap();
        assert_eq!(status, E2EStatus::Ok);
    }

    #[test]
    fn test_profile11_check_crc_error() {
        let config = Profile11Config {
            mode: Profile11IdMode::Both(0x1234),
            ..Default::default()
        };
        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);
        
        // Create and protect data
        let mut data = vec![0x00, 0x00, 0x11, 0x22];
        profile_tx.protect(&mut data).unwrap();
        
        // Corrupt the data
        data[2] = 0xFF;
        
        // Check should detect CRC error
        let status = profile_rx.check(&data).unwrap();
        assert_eq!(status, E2EStatus::CrcError);
    }

    #[test]
    fn test_profile11_check_data_id_error() {
        let config_tx = Profile11Config {
            mode: Profile11IdMode::Nibble(0x1320),
            ..Default::default()
        };
        let config_rx = Profile11Config {
            mode: Profile11IdMode::Nibble(0x1420),
            ..Default::default()
        };
        let mut profile_tx = Profile11::new(config_tx);
        let mut profile_rx = Profile11::new(config_rx);
        
        // Create and protect data
        let mut data = vec![0x00, 0x00, 0x55, 0x66];
        profile_tx.protect(&mut data).unwrap();
        
        // Check should detect Data ID error
        let status = profile_rx.check(&data).unwrap();
        assert_eq!(status, E2EStatus::DataIdError);
    }

    #[test]
    fn test_profile11_counter_sequence() {
        let config = Profile11Config::default();
        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);
        
        // Send multiple messages
        for i in 1..=20 {
            let mut data = vec![0x00, 0x00, i as u8];
            profile_tx.protect(&mut data).unwrap();
            
            let status = profile_rx.check(&data).unwrap();
            assert_eq!(status, E2EStatus::Ok);
            
            // Check counter value
            assert_eq!(data[1] & LOWER_NIBBLE_MASK, (i%COUNTER_MASK) as u8);
        }
    }

    #[test]
    fn test_profile11_counter_wrap_around() {
        let config = Profile11Config::default();
        let mut profile = Profile11::new(config);
        
        // Set counter to near maximum
        profile.counter = MAX_COUNTER_VALUE;
        
        let mut data1 = vec![0x00, 0x00, 0x01];
        profile.protect(&mut data1).unwrap();
        assert_eq!(data1[1] & LOWER_NIBBLE_MASK, 0); // Counter = 0
        
        let mut data2 = vec![0x00, 0x00, 0x02];
        profile.protect(&mut data2).unwrap();
        assert_eq!(data2[1] & LOWER_NIBBLE_MASK, 1); // Counter wraps to 1
    }

    #[test]
    fn test_profile11_buffer_too_small() {
        let config = Profile11Config::default();
        let mut profile = Profile11::new(config);
        
        let mut data = vec![0x00]; // Only 1 byte
        let result = profile.protect(&mut data);
        
        assert!(result.is_err());

        if let Err(E2EError::InvalidDataFormat(msg)) = result {
            assert!(msg.contains("below minimum required length of"));
        } else {
            panic!("Expected InvalidDataFormat error for too small buffer");
        }
    }

    #[test]
    fn test_profile11_buffer_too_long() {
        let config = Profile11Config {
            max_data_length : 5,
            ..Default::default()
        };
        let mut profile = Profile11::new(config);
        
        let mut data = vec![0x00, 0x00, 0x11, 0x22, 0x33, 0x44]; // Only 1 byte
        let result = profile.protect(&mut data);
        
        assert!(result.is_err());

        if let Err(E2EError::InvalidDataFormat(msg)) = result {
            assert!(msg.contains("exceeds configured maximum of"));
        } else {
            panic!("Expected InvalidDataFormat error for too long buffer");
        }
    }

    #[test]
    fn test_profile11_ok_some_lost() {
        let config = Profile11Config {
            max_delta_counter : 2,
            ..Default::default()
        };

        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);

        let mut data1 = vec![0x00, 0x00, 0x11];
        profile_tx.protect(&mut data1).unwrap();
        assert_eq!(profile_rx.check(&data1).unwrap(), E2EStatus::Ok);

        let mut data3 = vec![0x00, 0x00, 0x33];
        profile_tx.protect(&mut data3).unwrap();
        profile_tx.protect(&mut data3).unwrap();
        let status = profile_rx.check(&data3).unwrap();
        assert_eq!(status, E2EStatus::OkSomeLost);
    }

    #[test]
    fn test_profile11_wrong_sequence() {
        let config = Profile11Config {
            max_delta_counter : 2,
            ..Default::default()
        };

        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);

        let mut data1 = vec![0x00, 0x00, 0x11];
        profile_tx.protect(&mut data1).unwrap();
        assert_eq!(profile_rx.check(&data1).unwrap(), E2EStatus::Ok);

        let mut data3 = vec![0x00, 0x00, 0x33];
        profile_tx.protect(&mut data3).unwrap();
        profile_tx.protect(&mut data3).unwrap();
        profile_tx.protect(&mut data3).unwrap();
        let status = profile_rx.check(&data3).unwrap();
        assert_eq!(status, E2EStatus::WrongSequence);
    }

    #[test]
    fn test_profile11_repeated() {
        let config = Profile11Config {
            max_delta_counter : 2,
            ..Default::default()
        };

        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);

        let mut data1 = vec![0x00, 0x00, 0x11];
        profile_tx.protect(&mut data1).unwrap();
        assert_eq!(profile_rx.check(&data1).unwrap(), E2EStatus::Ok);

        let status = profile_rx.check(&data1).unwrap();
        assert_eq!(status, E2EStatus::Repeated);
    }

    #[test]
    #[should_panic(expected = "Invalid Profile11 configuration")]
    fn test_profile11_wrong_configuration_max_length() {
        let config = Profile11Config {
            max_data_length: DEFAULT_MAX_DATA_LENGTH + 1, // exceeds DEFAULT_MAX_DATA_LENGTH
            ..Default::default()
        };
        // panic!
        Profile11::new(config);
    }

    #[test]
    #[should_panic(expected = "Invalid Profile11 configuration")]
    fn test_profile11_wrong_configuration_delta() {
        let config = Profile11Config {
            max_delta_counter: 0, // 0 is not valid
            ..Default::default()
        };
        // panic!
        Profile11::new(config);
    }
    #[test]
    fn test_profile11_autosar_example() {
        let config = Profile11Config {
            max_delta_counter : 1,
            mode : Profile11IdMode::Both(0x123),
            ..Default::default()
        };

        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);

        let mut data1 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        profile_tx.protect(&mut data1).unwrap();
        assert_eq!(data1[0], 0x91);
        assert_eq!(profile_rx.check(&data1).unwrap(), E2EStatus::Ok);
    }
}