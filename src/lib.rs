//! # AUTOSAR E2E Protection Library
//!
//! This library implements the AUTOSAR E2E (End-to-End) protection mechanism
//! as specified in the AUTOSAR standard.
//!
//! ## Overview
//!
//! The E2E protection mechanism provides end-to-end data protection for
//! safety-critical automotive communication. It detects errors in data
//! transmission including:
//! - Data corruption (via CRC)
//! - Message loss, duplication, or reordering (via sequence counter)
//! - Incorrect addressing (via Data ID)
//!
//! ## Example
//!
//! ```rust
//! use autosar_e2e::{E2EProfile, E2EResult};
//! use autosar_e2e::profile11::{Profile11, Profile11Config, Profile11IdMode};
//!
//! // Create a Profile 11 configuration
//! let config = Profile11Config {
//!     mode: Profile11IdMode::Nibble(0x1A34),
//!     max_delta_counter: 1,
//!     ..Default::default()
//! };
//!
//! // Create the profile instance
//! let mut profile = Profile11::new(config);
//!
//! // Protect data
//! let mut data = vec![0x00, 0x00, 0x12, 0x34, 0x56]; //[CRC, counter, user data ..]
//! profile.protect(&mut data).unwrap();
//!
//! // Check protected data
//! let status = profile.check(&data).unwrap();
//! ```

use thiserror::Error;
pub mod profile11;

/// Result type for E2E operations
pub type E2EResult<T> = Result<T, E2EError>;

/// E2E Protection status enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E2EStatus {
    /// The checks of data in this cycle is successful
    Ok,
    /// CRC check failed - data corruption detected
    CrcError,
    /// Data ID check failed - incorrect addressing
    DataIdError,
    // Counter check failed - same counter as previous cycle 
    Repeated,
    // Counter check failed - counter is increated within allowed configured delta
    OkSomeLost,
    /// Counter check failed - possible message loss/duplication
    WrongSequence,
}

/// E2E Error types
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum E2EError {
    /// Invalid configuration provided
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
        
    /// Invalid data format
    #[error("Invalid data format: {0}")]
    InvalidDataFormat(String),
    
    /// Profile-specific error
    #[error("Profile-specific error: {0}")]
    ProfileSpecificError(String),
}

// Main trait for E2E Profile implementations
///
/// This trait defines the common interface that all E2E profiles must implement.
/// Each profile provides three main operations:
/// - `protect`: Add E2E protection to data
/// - `check`: Verify E2E protection on received data
/// - `forward`: Forward protected data (Profile 11 specific)
pub trait E2EProfile {
    /// Configuration type for this profile
    type Config;

    /// Create a new instance with the given configuration
    fn new(config: Self::Config) -> Self;

    /// Add E2E protection to the given data buffer
    ///
    /// This function modifies the data buffer in-place by adding:
    /// - CRC checksum
    /// - Sequence counter
    /// - Data ID (if applicable)
    ///
    /// # Arguments
    /// * `data` - Mutable reference to the data buffer to protect
    ///
    /// # Returns
    /// * `Ok(())` if protection was successfully added
    /// * `Err(E2EError)` if an error occurred
    fn protect(&mut self, data: &mut [u8]) -> E2EResult<()>;

    /// Check E2E protection on received data
    ///
    /// This function verifies the integrity of the received data by checking:
    /// - CRC checksum
    /// - Sequence counter continuity
    /// - Data ID (if applicable)
    ///
    /// # Arguments
    /// * `data` - Reference to the received data buffer
    ///
    /// # Returns
    /// * `Ok(E2EStatus)` indicating the check result
    /// * `Err(E2EError)` if an error occurred during checking
    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_e2e_status() {
        assert_eq!(E2EStatus::Ok, E2EStatus::Ok);
        assert_ne!(E2EStatus::Ok, E2EStatus::CrcError);
    }
}