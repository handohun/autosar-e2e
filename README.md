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
- **Profile 22** implementation
- **Profile 4** implementation
- **Profile 5** implementation
- **Profile 6** implementation
- **Profile 7** implementation
- **Profile 8** implementation
- Support for Protect, Check operations
- Comprehensive documentation and tests
- Configurable parameters per AUTOSAR specification

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
autosar-e2e = "0.4.0"
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

- [ ] Add Profile 4m support
- [ ] Add Profile 7m support
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
