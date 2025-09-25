use crate::{E2EError, E2EResult};
use std::fmt::Display;

pub fn validate_data_length_range<T>(len: T, min: T, max: T) -> E2EResult<()>
where
    T: PartialOrd + Display,
{
    if len < min || len > max {
        return Err(E2EError::InvalidDataFormat(format!(
            "Expected {} - {} bytes, got {} bytes",
            min, max, len
        )));
    }
    Ok(())
}

pub fn validate_data_length_range_u32(len: u32, min: u32, max: u32) -> E2EResult<()> {
    validate_data_length_range(len, min, max)
}

pub fn validate_data_length_exact(len: u16, expected: u16) -> E2EResult<()> {
    if len != expected {
        return Err(E2EError::InvalidDataFormat(format!(
            "Expected {} bytes, got {} bytes",
            expected, len
        )));
    }
    Ok(())
}

pub fn validate_min_data_length(data_length: u16, min_bytes: u16, max_bytes: u16) -> E2EResult<()> {
    if data_length < min_bytes || data_length > max_bytes {
        return Err(E2EError::InvalidConfiguration(format!(
            "Data length must be between {}B and {}B",
            min_bytes / 8,
            max_bytes / 8
        )));
    }
    Ok(())
}

pub fn validate_min_data_length_u32(data_length: u32, min_bytes: u32) -> E2EResult<()> {
    if data_length < min_bytes {
        return Err(E2EError::InvalidConfiguration(format!(
            "Minimum Data length shall be larger than {}B",
            min_bytes / 8
        )));
    }
    Ok(())
}

pub fn validate_max_data_length_u32(max_data_length: u32, min_data_length: u32) -> E2EResult<()> {
    if max_data_length < min_data_length {
        return Err(E2EError::InvalidConfiguration(
            "Maximum Data length shall be larger than MinDataLength".into(),
        ));
    }
    Ok(())
}

pub fn validate_counter_config<T>(max_delta_counter: T) -> E2EResult<()>
where
    T: PartialEq + Display + Copy,
    T: From<u8>,
{
    let zero = T::from(0);
    let max_val = match std::mem::size_of::<T>() {
        1 => T::from(u8::MAX),
        2 => unsafe { std::mem::transmute_copy(&u16::MAX) },
        4 => unsafe { std::mem::transmute_copy(&u32::MAX) },
        _ => panic!("Unsupported counter type"),
    };

    if max_delta_counter == zero || max_delta_counter == max_val {
        return Err(E2EError::InvalidConfiguration(format!(
            "Max delta counter must be between 1 and {}",
            max_val
        )));
    }
    Ok(())
}

pub fn validate_counter_config_u8(max_delta_counter: u8) -> E2EResult<()> {
    validate_counter_config(max_delta_counter)
}

pub fn validate_counter_config_u16(max_delta_counter: u16) -> E2EResult<()> {
    validate_counter_config(max_delta_counter)
}

pub fn validate_counter_config_u32(max_delta_counter: u32) -> E2EResult<()> {
    validate_counter_config(max_delta_counter)
}

pub fn validate_offset_within_data(
    offset: u16,
    data_length: u16,
    header_size: u16,
) -> E2EResult<()> {
    if data_length < header_size || offset > data_length - header_size {
        return Err(E2EError::InvalidConfiguration(format!(
            "Offset shall be between 0 and data length - {}B",
            header_size / 8
        )));
    }
    Ok(())
}
