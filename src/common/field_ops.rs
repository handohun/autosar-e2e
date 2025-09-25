pub fn read_be_u8_at(data: &[u8], offset: usize) -> u8 {
    data[offset]
}

pub fn read_be_u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([data[offset], data[offset + 1]])
}

pub fn read_be_u32_at(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

pub fn read_be_u64_at(data: &[u8], offset: usize) -> u64 {
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

pub fn read_le_u16_at(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

pub fn write_be_u8_at(data: &mut [u8], offset: usize, value: u8) {
    data[offset] = value;
}

pub fn write_be_u16_at(data: &mut [u8], offset: usize, value: u16) {
    data[offset..=offset + 1].copy_from_slice(&value.to_be_bytes());
}

pub fn write_be_u32_at(data: &mut [u8], offset: usize, value: u32) {
    data[offset..=offset + 3].copy_from_slice(&value.to_be_bytes());
}

pub fn write_be_u64_at(data: &mut [u8], offset: usize, value: u64) {
    data[offset..=offset + 7].copy_from_slice(&value.to_be_bytes());
}

pub fn write_le_u16_at(data: &mut [u8], offset: usize, value: u16) {
    data[offset..=offset + 1].copy_from_slice(&value.to_le_bytes());
}

/// Generic calculate offset bytes from bit offset
pub fn calculate_offset_bytes<T>(bit_offset: T) -> usize
where
    T: Into<u64>,
{
    (bit_offset.into() / 8) as usize
}

/// Calculate byte offset from bit offset (16-bit) - backward compatibility
pub fn calculate_offset_bytes_u16(bit_offset: u16) -> usize {
    calculate_offset_bytes(bit_offset)
}

/// Calculate byte offset from bit offset (32-bit) - backward compatibility
pub fn calculate_offset_bytes_u32(bit_offset: u32) -> usize {
    calculate_offset_bytes(bit_offset)
}

/// Generic nibble operations for Profile 11 and similar profiles
pub fn write_nibble_at(data: &mut [u8], bit_offset: u8, value: u8) {
    let byte_idx = (bit_offset >> 3) as usize;
    let shift = bit_offset & 0x07;
    let mask = !(0x0F << shift);
    let val = (value & 0x0F) << shift;
    data[byte_idx] = (data[byte_idx] & mask) | val;
}

pub fn read_nibble_at(data: &[u8], bit_offset: u8) -> u8 {
    let byte_idx = (bit_offset >> 3) as usize;
    let shift = bit_offset & 0x07;
    (data[byte_idx] >> shift) & 0x0F
}

/// Write counter value with mask (for Profile 22 style)
pub fn write_masked_u8_at(data: &mut [u8], offset: usize, value: u8, mask: u8) {
    data[offset] = (data[offset] & !mask) | (value & mask);
}

/// Generic field writer trait for compile-time optimization
pub trait FieldWriter<T> {
    fn write_at(data: &mut [u8], offset: usize, value: T);
    fn read_at(data: &[u8], offset: usize) -> T;
}

impl FieldWriter<u8> for u8 {
    fn write_at(data: &mut [u8], offset: usize, value: u8) {
        write_be_u8_at(data, offset, value);
    }

    fn read_at(data: &[u8], offset: usize) -> u8 {
        read_be_u8_at(data, offset)
    }
}

impl FieldWriter<u16> for u16 {
    fn write_at(data: &mut [u8], offset: usize, value: u16) {
        write_be_u16_at(data, offset, value);
    }

    fn read_at(data: &[u8], offset: usize) -> u16 {
        read_be_u16_at(data, offset)
    }
}

impl FieldWriter<u32> for u32 {
    fn write_at(data: &mut [u8], offset: usize, value: u32) {
        write_be_u32_at(data, offset, value);
    }

    fn read_at(data: &[u8], offset: usize) -> u32 {
        read_be_u32_at(data, offset)
    }
}

impl FieldWriter<u64> for u64 {
    fn write_at(data: &mut [u8], offset: usize, value: u64) {
        write_be_u64_at(data, offset, value);
    }

    fn read_at(data: &[u8], offset: usize) -> u64 {
        read_be_u64_at(data, offset)
    }
}
