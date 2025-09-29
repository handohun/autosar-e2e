# AUTOSAR E2E Protection Library

[![Crates.io](https://img.shields.io/crates/v/autosar-e2e.svg)](https://crates.io/crates/autosar-e2e)
[![Documentation](https://docs.rs/autosar-e2e/badge.svg)](https://docs.rs/autosar-e2e)
[![License](https://img.shields.io/crates/l/autosar-e2e.svg)](LICENSE)
[![CI](https://github.com/handohun/autosar-e2e/actions/workflows/ci.yml/badge.svg)](https://github.com/handohun/autosar-e2e/actions/workflows/ci.yml)

A **high-performance**, **memory-safe** Rust implementation of the AUTOSAR E2E (End-to-End) Protection Protocol for safety-critical automotive communication systems.

## Overview

This library implements the AUTOSAR E2E protection mechanism which provides **end-to-end data protection** for safety-critical automotive communication. The E2E protection helps detect:

| Protection Type | Detection Method | Status |
|----------------|------------------|--------|
| **Data Corruption** | CRC checksums | Implemented |
| **Message Loss/Duplication** | Sequence counters | Implemented |
| **Incorrect Addressing** | Data ID verification | Implemented |
| **Out-of-order Messages** | Counter validation | Implemented |

## Features

### Supported Profiles

| Profile | Description | CRC | Counter | Data ID | Status |
|---------|-------------|-----|---------|---------|--------|
| **Profile 4** | Large packets, low overhead | 32-bit | 16-bit | 32-bit | Complete |
| **Profile 4M** | Profile 4 + message metadata | 32-bit | 16-bit | 32-bit | Complete |
| **Profile 5** | Small packets, minimal overhead | 16-bit | 8-bit | 16-bit | Complete |
| **Profile 6** | Dynamic size data | 16-bit | 8-bit | 16-bit | Complete |
| **Profile 7** | High-integrity protection | 64-bit | 32-bit | 32-bit | Complete |
| **Profile 7M** | Profile 7 + message metadata | 64-bit | 32-bit | 32-bit | Complete |
| **Profile 8** | Flexible protection | 32-bit | 32-bit | 32-bit | Complete |
| **Profile 11** | Nibble/Both variants | 8-bit | 4-bit | Variable | Complete |
| **Profile 22** | Enhanced protection | 8-bit | 4-bit | Variable | Complete |

### Key Features

- **Zero-copy operations** - In-place data modification
- **Thread-safe** - All operations are safe for concurrent use
- **High performance** - Optimized common operations with shared helpers
- **Memory safe** - 100% safe Rust, no unsafe code
- **Configurable** - Extensive configuration options per AUTOSAR spec
- **Well tested** - Comprehensive test coverage including edge cases
- **Well documented** - Extensive API documentation with examples

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
autosar-e2e = "1.0.0"
```

## Quick Start

### Basic Usage

```rust
use autosar_e2e::{E2EProfile, E2EResult, E2EStatus};
use autosar_e2e::profile4::{Profile4, Profile4Config};

fn main() -> E2EResult<()> {
    // Configure Profile 4 for large data packets
    let config = Profile4Config {
        data_id: 0x12345678,
        max_delta_counter: 1,
        min_data_length: 96,    // 12 bytes minimum
        max_data_length: 4096,  // 512 bytes maximum
        ..Default::default()
    };

    // Create sender and receiver instances
    let mut sender = Profile4::new(config.clone())?;
    let mut receiver = Profile4::new(config)?;

    // Prepare data buffer (12 bytes minimum for Profile 4)
    let mut data = vec![0u8; 16]; // [length, counter, data_id, crc, user_data...]

    // Add E2E protection
    sender.protect(&mut data)?;
    println!("Protected data: {:02X?}", data);

    // Simulate network transmission...

    // Verify E2E protection at receiver
    match receiver.check(&data)? {
        E2EStatus::Ok => println!("Data integrity verified!"),
        E2EStatus::CrcError => println!("CRC error detected!"),
        E2EStatus::DataIdError => println!("Data ID mismatch!"),
        E2EStatus::WrongSequence => println!("Counter sequence error!"),
        E2EStatus::OkSomeLost => println!("Some messages lost but within tolerance"),
        E2EStatus::Repeated => println!("Repeated message detected"),
        E2EStatus::DataLengthError => println!("Data length mismatch!"),
    }

    Ok(())
}
```

### Advanced Configuration

```rust
use autosar_e2e::profile7::{Profile7, Profile7Config};

// High-integrity protection with 64-bit CRC
let config = Profile7Config {
    data_id: 0x0a0b0c0d,
    offset: 64,                    // Header at bit offset 64
    min_data_length: 20 * 8,       // 20 bytes minimum
    max_data_length: 4096 * 8,     // 4KB maximum
    max_delta_counter: 5,          // Allow up to 5 lost messages
};

let mut profile = Profile7::new(config)?;
```

## Architecture

### Clean Module Organization

```
src/
├── lib.rs              # Main library interface
├── profiles/           # All E2E profile implementations
│   ├── profile4.rs     # Large packets, 32-bit CRC
│   ├── profile4m.rs    # Profile 4 + message metadata
│   ├── profile5.rs     # Small packets, 16-bit CRC
│   ├── profile6.rs     # Dynamic size, 16-bit CRC
│   ├── profile7.rs     # High integrity, 64-bit CRC
│   ├── profile7m.rs    # Profile 7 + message metadata
│   ├── profile8.rs     # Flexible protection, 32-bit CRC
│   ├── profile11.rs    # Nibble/Both variants
│   └── profile22.rs    # Enhanced protection
└── profiles.rs         # Profiles module
```

### Trait-Based Design

The library follows a clean trait-based design for extensibility:

```rust
pub trait E2EProfile {
    type Config;

    /// Create a new profile instance with configuration
    fn new(config: Self::Config) -> E2EResult<Self>;

    /// Add E2E protection to data buffer (in-place)
    fn protect(&mut self, data: &mut [u8]) -> E2EResult<()>;

    /// Verify E2E protection on received data
    fn check(&mut self, data: &[u8]) -> E2EResult<E2EStatus>;
}
```

## CI/CD Pipeline

### Build & Test Status

| Platform | Rust Version | Build | Test | Status |
|----------|-------------|-------|------|--------|
| **Linux** | Stable | Pass | Pass | ![Ubuntu](https://img.shields.io/badge/Ubuntu-Passing-brightgreen) |
| **Windows** | Stable | Pass | Pass | ![Windows](https://img.shields.io/badge/Windows-Passing-brightgreen) |
| **macOS** | Stable | Pass | Pass | ![macOS](https://img.shields.io/badge/macOS-Passing-brightgreen) |

### Automated Quality Checks

- **Code Formatting**: `rustfmt` ensures consistent code style
- **Linting**: `clippy` with zero warnings tolerance
- **Security Audit**: `cargo-audit` scans for known vulnerabilities
- **Test Coverage**: Comprehensive coverage reporting via `tarpaulin`
- **Performance**: Automated benchmark regression detection

## Testing

Run the comprehensive test suite:

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific profile tests
cargo test profile4

# Run tests in release mode for performance
cargo test --release
```

### Test Coverage

```bash
# Generate coverage report
cargo tarpaulin --out Html

# View coverage
open tarpaulin-report.html
```

**Current test coverage: 83%** with **32** test cases covering:

- Basic protection/check cycles
- Counter wraparound scenarios
- Error detection (CRC, sequence, length)
- Edge cases and boundary conditions
- Configuration validation

## Performance

The library is optimized for automotive real-time constraints with **continuous benchmarking** in CI:

### Latest Benchmark Results

| Profile | Data Size | Protect | Check | Memory | Status |
|---------|-----------|---------|-------|--------|--------|
| **Profile 4** | 16 bytes | 258ns | 225ns | Zero-copy | Pass |
| **Profile 5** | 8 bytes | 189ns | 161ns | Zero-copy | Pass |
| **Profile 7** | 28 bytes | 512ns | 474ns | Zero-copy | Pass |
| **Profile 11** | 5 bytes | 71ns | 46ns | Zero-copy | Pass |

*Benchmarks automatically run on every commit using GitHub Actions with Criterion.rs*

### Performance Scaling by Data Size

| Profile 4 | 16B | 64B | 256B | 1024B | Regression Alert |
|-----------|-----|-----|------|-------|------------------|
| **Protect** | 258ns | 335ns | 647ns | 1.92μs | Auto-detected |
| **Check** | 225ns | 300ns | 620ns | 1.88μs | Auto-detected |

### CI Performance Features

- **Automated Benchmarking**: Every commit triggers comprehensive performance tests
- **Trend Analysis**: Performance regression detection with 200% alert threshold
- **Historical Data**: Track performance changes over time via GitHub Actions
- **Multiple Profiles**: All E2E profiles benchmarked across different data sizes
- **Cross-Platform**: Benchmarks run on Ubuntu, Windows, and macOS
- **Real-time Updates**: Performance badges update automatically

The library demonstrates excellent scalability with sub-microsecond performance for typical automotive message sizes, making it suitable for real-time safety-critical systems. Performance regressions are automatically detected and reported in CI.

## Configuration Examples

### Profile Selection Guide

| Use Case | Recommended Profile | Reason |
|----------|-------------------|---------|
| **High-speed CAN** | Profile 5 | Minimal 3-byte overhead |
| **Ethernet backbone** | Profile 4 | Flexible length support |
| **Safety-critical** | Profile 7 | 64-bit CRC protection |
| **Telemetry** | Profile 8 | Large counter space |
| **Legacy systems** | Profile 11 | Compact nibble format |

### Sample Configurations

```rust
// Minimal overhead for CAN (Profile 5)
let can_config = Profile5Config {
    data_length: 8 * 8,    // 8 bytes total
    data_id: 0x123,
    max_delta_counter: 1,
    offset: 0,
};

// High-integrity Ethernet (Profile 7)
let eth_config = Profile7Config {
    min_data_length: 20 * 8,
    max_data_length: 1500 * 8,  // MTU size
    data_id: 0xdeadbeef,
    max_delta_counter: 10,      // Higher tolerance
};
```

## Safety and Correctness

- **Memory safety**: 100% safe Rust, no unsafe code
- **Static analysis**: Passes clippy with zero warnings
- **Fuzz tested**: Robust against malformed inputs
- **AUTOSAR compliant**: Follows specification exactly
- **Verified CRCs**: Uses industry-standard polynomials
- **Correct wraparound**: Handles counter overflow properly

### Security Considerations

```rust
// Data ID should be unique per message type
let config = ProfileConfig {
    data_id: 0x12345678,  // Good: unique per message
    // data_id: 0x00000000,  // Bad: not unique
};

// Configure appropriate counter tolerance
let config = ProfileConfig {
    max_delta_counter: 5,  // Good: allows some loss
    // max_delta_counter: 255, // Bad: too permissive
};
```

## Roadmap

### Completed

- [x] Core E2E profiles (4, 5, 6, 7, 8, 11, 22)
- [x] Comprehensive test coverage
- [x] Code refactoring and optimization
- [x] Documentation and examples
- [x] Profile 7M implementation
- [x] Profile 4M implementation
- [x] Peformance benchmarks

### Future

- [ ] code optimization

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

### Quick Start for Contributors

```bash
# Clone the repository
git clone https://github.com/handohun/autosar-e2e
cd autosar-e2e

# Run the complete CI pipeline locally
cargo bench                  # Run performance benchmarks
cargo test                   # Run all tests
cargo fmt --check           # Check code formatting
cargo clippy -- -D warnings # Run linter
cargo tarpaulin             # Generate coverage report
cargo audit                 # Security audit

# Generate documentation
cargo doc --open
```

### Local Development Workflow

```bash
# Run benchmarks with HTML reports
cargo bench

# Run specific benchmark
cargo bench profile4

# Compare benchmark results
cargo bench -- --baseline main

# Watch for changes and run tests
cargo watch -x test

# Check all quality gates (like CI)
cargo fmt --check && cargo clippy -- -D warnings && cargo test && cargo audit
```

## License

This project is dual-licensed under either:

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- **MIT license** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## References and Resources

- [AUTOSAR E2E Protocol Specification](https://www.autosar.org/fileadmin/standards/R20-11/FO/AUTOSAR_PRS_E2EProtocol.pdf)
- [AUTOSAR CRC Routines Specification](https://www.autosar.org/fileadmin/standards/R22-11/CP/AUTOSAR_SWS_CRCLibrary.pdf)
- [AUTOSAR Classic Platform](https://www.autosar.org/standards/classic-platform/)
- [Rust Embedded Working Group](https://github.com/rust-embedded/wg)

## Disclaimer

This is an **independent implementation** and is not officially affiliated with or endorsed by AUTOSAR GbR. The implementation follows the publicly available AUTOSAR specifications but has not undergone official AUTOSAR certification.

For **production safety-critical systems**, please ensure appropriate validation and testing according to your functional safety requirements (ISO 26262 or similar).

---

**Made with care for the automotive industry**

[Report Bug](https://github.com/handohun/autosar-e2e/issues) • [Request Feature](https://github.com/handohun/autosar-e2e/issues) • [Discussions](https://github.com/handohun/autosar-e2e/discussions)