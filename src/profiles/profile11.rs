//! # E2E Profile 11 Implementation
//!
//! Profile 11 is designed for protecting small data packets (up to MAX_DATA_LENGTH_BITS bytes)
//! with low overhead. It uses:
//! - 8-bit CRC for data integrity
//! - 4-bit counter for sequence checking (0-14)
//! - 4-bit Data ID nibble for addressing verification
//!
//! # Data layout
//! [DATA ... | CRC(1B) | HDR(1B) | DATA ...]
//! - HDR (bits 7..4) : DI_hi_nibble(nibble mode) OR data(both mode)
//! - HDR (bits 3..0) : counter
//!
//! # Modes
//!
//! Profile 11 supports two main modes:
//! - **Both(11A)**: full 16-bit Data-ID is implicit (only used in CRC).
//! - **Nibble(11C)**: high 4-bit is explicit in the header (1..=0xE recommended), low 8-bit is implicit (in CRC).

use crate::{E2EError, E2EProfile, E2EResult, E2EStatus};
use crc::{Algorithm, Crc};

// Constants
const NIBBLE_MASK: u8 = 0x0F;
const COUNTER_MAX: u8 = 14;
const COUNTER_MODULO: u8 = 15;
const MAX_DATA_LENGTH_BITS: u8 = 240;
const BITS_PER_BYTE: u8 = 8;
const BITS_PER_NIBBLE: u8 = 4;

// Profile 11 uses CRC-8-SAE-J1850 with custom parameters
const CRC8_ALGO: Algorithm<u8> = Algorithm {
    width: 8,
    poly: 0x1d,
    init: 0x00,
    refin: false,
    refout: false,
    xorout: 0x00,
    check: 0x4b,
    residue: 0xc4,
};

/// Data-ID mode for Profile 11.
///
/// # Variants
///
/// * `Both` - Profile 11A: The complete 16-bit Data-ID is only used
///   implicitly for CRC calculation. The header preserves the original
///   upper nibble of the data.
///
/// * `Nibble` - Profile 11C: The upper 4 bits of the Data-ID are
///   stored explicitly in the header, while the lower 8 bits are used
///   implicitly for CRC calculation. Recommended range: 0x100-0xE00.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile11IdMode {
    Both,
    Nibble, // Only lower 12 bits used: 0x000..=0xFFF
}

/// Configuration for E2E Profile 11
#[derive(Debug, Clone)]
pub struct Profile11Config {
    /// Bit offset of Counter in MSB first order
    pub counter_offset: u8,
    /// Bit offset of CRC in MSB first order
    pub crc_offset: u8,
    /// Profile mode(11A or 11C)
    pub mode: Profile11IdMode,
    /// A unique identifier
    pub data_id: u16,
    /// Bit offset of the low nibble of the high byte of Data ID
    pub nibble_offset: u8,
    /// Maximum allowed delta between consecutive counters
    pub max_delta_counter: u8,
    /// data length (up to DEFAULT_MAX_DATA_LENGTH bytes)
    pub data_length: u8,
}

impl Default for Profile11Config {
    fn default() -> Self {
        Self {
            counter_offset: 8, // bits
            crc_offset: 0,     // bits
            mode: Profile11IdMode::Nibble,
            data_id: 0x123,
            nibble_offset: 12, // bits
            max_delta_counter: 1,
            data_length: 64, // bits
        }
    }
}

pub struct Profile11Check {
    rx_counter: u8,
    rx_crc: u8,
    rx_nibble: u8,
    calculated_crc: u8,
}
/// E2E Profile 11 Implementation
///
/// Implements AUTOSAR E2E Profile 11 protection mechanism with support
/// for both 11A and 11C variants.
#[derive(Clone)]
pub struct Profile11 {
    config: Profile11Config,
    counter: u8,
    initialized: bool,
}

impl Profile11 {
    /// Validate configuration parameters
    fn validate_config(config: &Profile11Config) -> E2EResult<()> {
        if config.data_length > MAX_DATA_LENGTH_BITS {
            return Err(E2EError::InvalidConfiguration(format!(
                "Maximum data length for Profile 11 is {} bits",
                MAX_DATA_LENGTH_BITS
            )));
        }

        if !config.data_length.is_multiple_of(BITS_PER_BYTE) {
            return Err(E2EError::InvalidConfiguration(
                "Data length shall be a multiple of 8".into(),
            ));
        }

        if config.max_delta_counter == 0 || config.max_delta_counter > COUNTER_MAX {
            return Err(E2EError::InvalidConfiguration(format!(
                "Max delta counter must be between 1 and {}",
                COUNTER_MAX
            )));
        }

        if !config.counter_offset.is_multiple_of(BITS_PER_NIBBLE) {
            return Err(E2EError::InvalidConfiguration(
                "Counter offset shall be a multiple of 4".into(),
            ));
        }

        if !config.crc_offset.is_multiple_of(BITS_PER_BYTE) {
            return Err(E2EError::InvalidConfiguration(
                "Crc offset shall be a multiple of 8".into(),
            ));
        }

        if config.mode == Profile11IdMode::Nibble && !config.nibble_offset.is_multiple_of(4) {
            return Err(E2EError::InvalidConfiguration(
                "Nibble offset must be a multiple of 4 bits".into(),
            ));
        }

        Ok(())
    }
    /// Validate data length against min/max constraints
    fn validate_length(&self, len: usize) -> E2EResult<()> {
        let expected_bytes = (self.config.data_length / BITS_PER_BYTE) as usize;
        if len != expected_bytes {
            return Err(E2EError::InvalidDataFormat(format!(
                "Expected {} bytes, got {} bytes",
                expected_bytes, len
            )));
        }
        Ok(())
    }
    fn write_nibble_data(&self, offset: u8, set_value: u8, data: &mut [u8]) {
        let byte_idx = (offset >> 3) as usize;
        let shift = offset & 0x07;

        let mask = !(NIBBLE_MASK << shift);
        let val = (set_value & NIBBLE_MASK) << shift;
        data[byte_idx] = (data[byte_idx] & mask) | val;
    }
    fn read_nibble_data(&self, offset: u8, data: &[u8]) -> u8 {
        let byte_idx = (offset >> 3) as usize;
        let shift = offset & 0x07;

        (data[byte_idx] >> shift) & NIBBLE_MASK
    }
    fn write_crc(&self, calculated_crc: u8, data: &mut [u8]) {
        let byte_position = (self.config.crc_offset / BITS_PER_BYTE) as usize;
        data[byte_position] = calculated_crc;
    }
    fn read_crc(&self, data: &[u8]) -> u8 {
        let byte_position = (self.config.crc_offset / BITS_PER_BYTE) as usize;
        data[byte_position]
    }
    /// Update Crc with ID
    fn update_crc_with_id(&self, digest: &mut crc::Digest<u8>) {
        match self.config.mode {
            Profile11IdMode::Both => {
                digest.update(&self.config.data_id.to_le_bytes());
            }
            Profile11IdMode::Nibble => {
                digest.update(&[self.config.data_id.to_le_bytes()[0], 0x00]);
            }
        }
    }
    fn update_crc_with_data(&self, digest: &mut crc::Digest<u8>, data: &[u8]) {
        if self.config.crc_offset > 0 {
            let offset_byte = (self.config.crc_offset / BITS_PER_BYTE) as usize;
            digest.update(&data[0..offset_byte]);
            digest.update(&data[(offset_byte + 1)..]);
        } else {
            digest.update(&data[1..]);
        }
    }
    fn compute_crc(&self, data: &[u8]) -> u8 {
        let crc: Crc<u8> = Crc::<u8>::new(&CRC8_ALGO);
        let mut digest = crc.digest();
        self.update_crc_with_id(&mut digest);
        self.update_crc_with_data(&mut digest, data);
        digest.finalize()
    }
    fn increment_counter(&mut self) {
        self.counter = (self.counter + 1) % COUNTER_MODULO;
    }
    fn do_checks(&mut self, check_items: Profile11Check) -> E2EStatus {
        if check_items.calculated_crc != check_items.rx_crc {
            return E2EStatus::CrcError;
        }
        if (self.config.mode == Profile11IdMode::Nibble)
            && ((self.config.data_id >> BITS_PER_BYTE) as u8 & NIBBLE_MASK) != check_items.rx_nibble
        {
            return E2EStatus::DataIdError;
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

impl E2EProfile for Profile11 {
    type Config = Profile11Config;

    fn new(config: Self::Config) -> Self {
        // Validate config (panic if invalid in constructor for simplicity)
        Self::validate_config(&config).expect("Invalid Profile11 configuration");
        Self {
            config,
            counter: 0,
            initialized: false,
        }
    }

    fn protect(&mut self, data: &mut [u8]) -> E2EResult<()> {
        self.validate_length(data.len())?;
        if self.config.mode == Profile11IdMode::Nibble {
            self.write_nibble_data(
                self.config.nibble_offset,
                self.config.data_id.to_le_bytes()[1],
                data,
            );
        }
        self.write_nibble_data(self.config.counter_offset, self.counter, data);
        let calculated_crc = self.compute_crc(data);
        self.write_crc(calculated_crc, data);
        self.increment_counter();
        Ok(())
    }

    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus> {
        // Check data length
        self.validate_length(data.len())?;
        let check_items = Profile11Check {
            rx_nibble: self.read_nibble_data(self.config.nibble_offset, data),
            rx_counter: self.read_nibble_data(self.config.counter_offset, data),
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
    fn test_profile11_basic_both_example() {
        let config = Profile11Config {
            max_delta_counter: 1,
            mode: Profile11IdMode::Both,
            data_id: 0x123,
            ..Default::default()
        };

        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);

        let mut data1 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        profile_tx.protect(&mut data1).unwrap();
        assert_eq!(data1[0], 0xcc);
        assert_eq!(data1[1], 0x00);
        assert_eq!(profile_rx.check(&data1).unwrap(), E2EStatus::Ok);

        profile_tx.protect(&mut data1).unwrap();
        assert_eq!(data1[0], 0x91);
        assert_eq!(data1[1], 0x01);
        assert_eq!(profile_rx.check(&data1).unwrap(), E2EStatus::Ok);
    }

    #[test]
    fn test_profile11_basic_nibble_example() {
        let config = Profile11Config {
            max_delta_counter: 1,
            mode: Profile11IdMode::Nibble,
            data_id: 0x123,
            ..Default::default()
        };

        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);

        let mut data1 = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        profile_tx.protect(&mut data1).unwrap();
        assert_eq!(data1[0], 0x2a);
        assert_eq!(data1[1], 0x10);
        assert_eq!(profile_rx.check(&data1).unwrap(), E2EStatus::Ok);

        profile_tx.protect(&mut data1).unwrap();
        assert_eq!(data1[0], 0x77);
        assert_eq!(data1[1], 0x11);
        assert_eq!(profile_rx.check(&data1).unwrap(), E2EStatus::Ok);
    }
    #[test]
    fn test_profile11_offset_nibble_example() {
        let config = Profile11Config {
            max_delta_counter: 1,
            crc_offset: 64,
            counter_offset: 72,
            nibble_offset: 76,
            data_length: 128,
            mode: Profile11IdMode::Nibble,
            data_id: 0x123,
        };

        let mut profile_tx = Profile11::new(config.clone());
        let mut profile_rx = Profile11::new(config);

        let mut data1 = vec![
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        profile_tx.protect(&mut data1).unwrap();
        assert_eq!(data1[8], 0x7d);
        assert_eq!(data1[9], 0x10);
        assert_eq!(profile_rx.check(&data1).unwrap(), E2EStatus::Ok);
    }
    #[test]
    fn test_profile11_crc_error() {
        let mut profile = Profile11::new(Profile11Config::default());
        let mut data = vec![0x00; 8];
        profile.protect(&mut data).unwrap();
        data[0] ^= 0xFF;
        assert_eq!(profile.check(&data).unwrap(), E2EStatus::CrcError);
    }
    #[test]
    fn test_profile11_counter_wraparound() {
        let mut profile = Profile11::new(Profile11Config::default());
        let mut data = vec![0x00; 8];
        for _ in 0..=COUNTER_MAX + 1 {
            profile.protect(&mut data).unwrap();
        }
        assert_eq!(
            profile.read_nibble_data(profile.config.counter_offset, &data),
            0x00
        );
    }
    #[test]
    fn test_profile11_some_lost_ok() {
        let config = Profile11Config {
            max_delta_counter: 3,
            ..Default::default()
        };
        let mut tx = Profile11::new(config.clone());
        let mut rx = Profile11::new(config);

        let mut data = vec![0x00; 8];
        tx.protect(&mut data).unwrap();
        rx.check(&data).unwrap();

        // Counter jump
        tx.increment_counter();
        tx.protect(&mut data).unwrap();
        assert_eq!(rx.check(&data).unwrap(), E2EStatus::OkSomeLost);
    }
    #[test]
    fn test_profile11_wrong_sequence() {
        let config = Profile11Config {
            max_delta_counter: 1,
            ..Default::default()
        };
        let mut tx = Profile11::new(config.clone());
        let mut rx = Profile11::new(config);

        let mut data = vec![0x00; 8];
        tx.protect(&mut data).unwrap();
        rx.check(&data).unwrap();

        // Counter jump a lot
        tx.counter = (tx.counter + 3) % COUNTER_MODULO;
        tx.protect(&mut data).unwrap();
        assert_eq!(rx.check(&data).unwrap(), E2EStatus::WrongSequence);
    }
    #[test]
    fn test_profile11_repeated_frame() {
        let mut profile = Profile11::new(Profile11Config::default());
        let mut profile_rx = profile.clone();
        let mut data = vec![0x00; 8];
        profile.protect(&mut data).unwrap();
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Ok);
        assert_eq!(profile_rx.check(&data).unwrap(), E2EStatus::Repeated);
    }
}
