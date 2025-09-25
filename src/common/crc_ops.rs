use crc::{Algorithm, Crc};

// Re-export CRC algorithms for easy access
pub use crc::{CRC_16_IBM_3740, CRC_32_AUTOSAR, CRC_64_XZ, CRC_8_AUTOSAR};

/// Profile 11 uses CRC-8-SAE-J1850 with custom parameters
pub const CRC8_PROFILE11: Algorithm<u8> = Algorithm {
    width: 8,
    poly: 0x1d,
    init: 0x00,
    refin: false,
    refout: false,
    xorout: 0x00,
    check: 0x4b,
    residue: 0xc4,
};

/// Compute CRC-8 for Profile 11
pub fn compute_crc8_profile11(segments: &[&[u8]]) -> u8 {
    let crc = Crc::<u8>::new(&CRC8_PROFILE11);
    let mut digest = crc.digest();
    for segment in segments {
        digest.update(segment);
    }
    digest.finalize()
}

/// Compute CRC-8 AUTOSAR for Profile 22
pub fn compute_crc8_autosar(segments: &[&[u8]]) -> u8 {
    let crc = Crc::<u8>::new(&CRC_8_AUTOSAR);
    let mut digest = crc.digest();
    for segment in segments {
        digest.update(segment);
    }
    digest.finalize()
}

/// Compute CRC-16 IBM 3740 for Profiles 4, 5, 6
pub fn compute_crc16_ibm3740(segments: &[&[u8]]) -> u16 {
    let crc = Crc::<u16>::new(&CRC_16_IBM_3740);
    let mut digest = crc.digest();
    for segment in segments {
        digest.update(segment);
    }
    digest.finalize()
}

/// Compute CRC-16 IBM 3740 with data ID for Profile 5/6 style
pub fn compute_crc16_ibm3740_with_data_id(segments: &[&[u8]], data_id: &[u8]) -> u16 {
    let crc = Crc::<u16>::new(&CRC_16_IBM_3740);
    let mut digest = crc.digest();
    for segment in segments {
        digest.update(segment);
    }
    digest.update(data_id);
    digest.finalize()
}

/// Compute CRC-32 AUTOSAR for Profile 8
pub fn compute_crc32_autosar(segments: &[&[u8]]) -> u32 {
    let crc = Crc::<u32>::new(&CRC_32_AUTOSAR);
    let mut digest = crc.digest();
    for segment in segments {
        digest.update(segment);
    }
    digest.finalize()
}

/// Compute CRC-64 XZ for Profile 7
pub fn compute_crc64_xz(segments: &[&[u8]]) -> u64 {
    let crc = Crc::<u64>::new(&CRC_64_XZ);
    let mut digest = crc.digest();
    for segment in segments {
        digest.update(segment);
    }
    digest.finalize()
}

/// Helper function to compute CRC-16 excluding a specific byte range
pub fn compute_crc16_exclude_range(
    algorithm: &'static Algorithm<u16>,
    data: &[u8],
    exclude_start: usize,
    exclude_len: usize,
) -> u16 {
    let crc = Crc::<u16>::new(algorithm);
    let mut digest = crc.digest();

    if exclude_start > 0 {
        digest.update(&data[0..exclude_start]);
    }

    let exclude_end = exclude_start + exclude_len;
    if exclude_end < data.len() {
        digest.update(&data[exclude_end..]);
    }

    digest.finalize()
}

/// Helper function to compute CRC-32 excluding a specific byte range
pub fn compute_crc32_exclude_range(
    algorithm: &'static Algorithm<u32>,
    data: &[u8],
    exclude_start: usize,
    exclude_len: usize,
) -> u32 {
    let crc = Crc::<u32>::new(algorithm);
    let mut digest = crc.digest();

    if exclude_start > 0 {
        digest.update(&data[0..exclude_start]);
    }

    let exclude_end = exclude_start + exclude_len;
    if exclude_end < data.len() {
        digest.update(&data[exclude_end..]);
    }

    digest.finalize()
}

/// Helper function to compute CRC-64 excluding a specific byte range
pub fn compute_crc64_exclude_range(
    algorithm: &'static Algorithm<u64>,
    data: &[u8],
    exclude_start: usize,
    exclude_len: usize,
) -> u64 {
    let crc = Crc::<u64>::new(algorithm);
    let mut digest = crc.digest();

    if exclude_start > 0 {
        digest.update(&data[0..exclude_start]);
    }

    let exclude_end = exclude_start + exclude_len;
    if exclude_end < data.len() {
        digest.update(&data[exclude_end..]);
    }

    digest.finalize()
}
