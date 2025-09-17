# autosar-e2e

[![Crates.io](https://img.shields.io/crates/v/autosar-e2e.svg)](https://crates.io/crates/autosar-e2e)
[![Documentation](https://docs.rs/autosar-e2e/badge.svg)](https://docs.rs/autosar-e2e)
[![License](https://img.shields.io/crates/l/autosar-e2e.svg)](LICENSE)

A Rust implementation of the AUTOSAR E2E (End-to-End) Protection Protocol.

## Overview

This library implements the AUTOSAR E2E protection mechanism which provides end-to-end data protection for safety-critical automotive communication systems. The E2E protection helps detect:

- **Data Corruption**: Through CRC checksums
- **Message Loss/Duplication**: Through sequence counters
- **Incorrect Addressing**: Through Data ID verification

## Features

- **Profile 11** implementation (variants 11A and 11C)
- Support for Protect, Check operations
- Comprehensive documentation and tests
- Configurable parameters per AUTOSAR specification

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
autosar-e2e = "0.1.0"
```

## Usage

### Basic Example

```rust
use autosar_e2e::{E2EProfile, E2EResult, E2EStatus};
use autosar_e2e::profile11::{Profile11, Profile11Config, Profile11Variant};

fn main() -> E2EResult<()> {
    // Configure Profile 11
    let config = Profile11Config {
        mode: Profile11IdMode::Nibble,
        data_id : 0x123,
        max_delta_counter: 1,
        ..Default::default()
    };

    // Create sender and receiver instances
    let mut sender = Profile11::new(config.clone());
    let mut receiver = Profile11::new(config);

    // Prepare data to send
    let mut data = vec![0x00, 0x00, 0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC]; // [CRC, counter, user data ..]
    
    // Add E2E protection
    sender.protect(&mut data)?;
    println!("Protected data: {:02X?}", data);

    // Simulate transmission...
    
    // Verify E2E protection at receiver
    match receiver.check(&data)? {
        E2EStatus::Ok => println!("Data integrity verified!"),
        E2EStatus::CrcError => println!("CRC error detected!"),
        E2EStatus::DataIdError => println!("Data ID mismatch!"),
        _ => println!("Counter sequence error!"),
    }

    Ok(())
}
```

## Profile 11 Specifications

### Data Layout

Basically, Profile 11 uses a 2-byte header. If crc offset is zero, data layout is as the below.
```
Byte 0: CRC-8
Byte 1: [DataIDNibble(4 bits) | Counter(4 bits)]
Bytes 2-n: User Data
```

If crc offset is 64 and distance among crc, nibble and counter is the same, data layout is as the below.
```
Byte 0-7: User Data
Byte 8: CRC-8
Byte 9: [DataIDNibble(4 bits) | Counter(4 bits)]
Bytes 10-n: User Data
```

### Modes

- **Both(11A)**: DataIDNibble field can be used for user data
- **Nibble(11C)**: DataIDNibble field is used for protection

### Configuration Parameters

| Parameter | Description | Range | Default |
|-----------|-------------|-------|---------|
|`counter_offset`| bit offset of counter(it shall be a multiple of 4) | - | 8 |
|`crc_offset`| bit offset of crc(it shall be a multiple of 8) | - | 0 |
| `mode` | Profile mode | Both or Nibble | Nibble |
|`data_id`| bit offset of nibble(it shall be a multiple of 4) | - | 0x0123 |
|`nibble_offset`| bit offset of data id nibble(it shall be a multiple of 4) | - | 12 |
| `max_delta_counter` | Maximum counter delta | 1-14 | 1 |
| `data_length` | data length bits(It shall be a multiple of 8) | ≤240 | 64 |

## Architecture

The library follows a trait-based design for extensibility:

```rust
pub trait E2EProfile {
    type Config;
    
    fn new(config: Self::Config) -> Self;
    fn protect(&mut self, data: &mut Vec<u8>) -> E2EResult<()>;
    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus>;
}
```

This design allows for easy addition of other E2E profiles in the future.

## Testing

Run the test suite:

```bash
cargo test
```

Run tests with coverage:

```bash
cargo tarpaulin --out Html
```

## Safety and Correctness

- All CRC calculations use the standard `crc` crate with AUTOSAR polynomial
- Counter wrap-around is handled correctly
- Comprehensive test coverage including edge cases
- No unsafe code

## Future Work

- [ ] Add Profile 1 support
- [ ] Add Profile 2 support
- [ ] Add Profile 4 support
- [ ] Add Profile 5 support
- [ ] Add Profile 6 support
- [ ] Add Profile 7 support
- [ ] Add Profile 22 support
- [ ] Performance benchmarks
- [ ] Async support

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## References

- [AUTOSAR E2E Protocol Specification](https://www.autosar.org/)
- [AUTOSAR Classic Platform](https://www.autosar.org/standards/classic-platform/)

## Disclaimer

This is an independent implementation and is not officially affiliated with or endorsed by AUTOSAR.