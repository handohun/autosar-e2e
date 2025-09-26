use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use autosar_e2e::{E2EProfile, E2EResult};
use autosar_e2e::profile4::{Profile4, Profile4Config};
use autosar_e2e::profile5::{Profile5, Profile5Config};
use autosar_e2e::profile6::{Profile6, Profile6Config};
use autosar_e2e::profile7::{Profile7, Profile7Config};
use autosar_e2e::profile8::{Profile8, Profile8Config};
use autosar_e2e::profile11::{Profile11, Profile11Config, Profile11IdMode};
use autosar_e2e::profile22::{Profile22, Profile22Config};

fn benchmark_profile4(c: &mut Criterion) {
    let config = Profile4Config {
        data_id: 0x12345678,
        max_delta_counter: 1,
        min_data_length: 96,    // 12 bytes minimum
        max_data_length: 32768,  // 4096 bytes maximum
        ..Default::default()
    };

    let mut sender = Profile4::new(config.clone());
    let mut receiver = Profile4::new(config);

    let mut group = c.benchmark_group("Profile4");

    for size in &[16, 64, 256, 1024] {
        let mut data = vec![0u8; *size];

        group.bench_with_input(BenchmarkId::new("protect", size), size, |b, &_size| {
            b.iter(|| {
                let mut data_copy = data.clone();
                sender.protect(black_box(&mut data_copy)).unwrap();
            })
        });

        // Prepare protected data for check benchmark
        sender.protect(&mut data).unwrap();

        group.bench_with_input(BenchmarkId::new("check", size), size, |b, &_size| {
            b.iter(|| {
                receiver.check(black_box(&data)).unwrap();
            })
        });
    }

    group.finish();
}

fn benchmark_profile5(c: &mut Criterion) {
    let config = Profile5Config {
        data_length: 8 * 8,    // 8 bytes total
        data_id: 0x123,
        max_delta_counter: 1,
        offset: 0,
    };

    let mut sender = Profile5::new(config.clone());
    let mut receiver = Profile5::new(config);

    let mut group = c.benchmark_group("Profile5");
    let mut data = vec![0u8; 8];

    group.bench_function("protect", |b| {
        b.iter(|| {
            let mut data_copy = data.clone();
            sender.protect(black_box(&mut data_copy)).unwrap();
        })
    });

    // Prepare protected data for check benchmark
    sender.protect(&mut data).unwrap();

    group.bench_function("check", |b| {
        b.iter(|| {
            receiver.check(black_box(&data)).unwrap();
        })
    });

    group.finish();
}

fn benchmark_profile6(c: &mut Criterion) {
    let config = Profile6Config {
        data_id: 0x1234,
        offset: 0,
        min_data_length: 32*8,
        max_data_length: 256*8,
        max_delta_counter: 1,
    };

    let mut sender = Profile6::new(config.clone());
    let mut receiver = Profile6::new(config);

    let mut group = c.benchmark_group("Profile6");

    for size in &[32, 64, 128, 256] {
        let mut data = vec![0u8; *size];

        group.bench_with_input(BenchmarkId::new("protect", size), size, |b, &_size| {
            b.iter(|| {
                let mut data_copy = data.clone();
                sender.protect(black_box(&mut data_copy)).unwrap();
            })
        });

        // Prepare protected data for check benchmark
        sender.protect(&mut data).unwrap();

        group.bench_with_input(BenchmarkId::new("check", size), size, |b, &_size| {
            b.iter(|| {
                receiver.check(black_box(&data)).unwrap();
            })
        });
    }

    group.finish();
}

fn benchmark_profile7(c: &mut Criterion) {
    let config = Profile7Config {
        data_id: 0x0a0b0c0d,
        offset: 64,
        min_data_length: 20 * 8,
        max_data_length: 4096 * 8,
        max_delta_counter: 5,
    };

    let mut sender = Profile7::new(config.clone());
    let mut receiver = Profile7::new(config);

    let mut group = c.benchmark_group("Profile7");

    for size in &[28, 64, 256, 1024] {
        let mut data = vec![0u8; *size];

        group.bench_with_input(BenchmarkId::new("protect", size), size, |b, &_size| {
            b.iter(|| {
                let mut data_copy = data.clone();
                sender.protect(black_box(&mut data_copy)).unwrap();
            })
        });

        // Prepare protected data for check benchmark
        sender.protect(&mut data).unwrap();

        group.bench_with_input(BenchmarkId::new("check", size), size, |b, &_size| {
            b.iter(|| {
                receiver.check(black_box(&data)).unwrap();
            })
        });
    }

    group.finish();
}

fn benchmark_profile8(c: &mut Criterion) {
    let config = Profile8Config {
        data_id: 0x12345678,
        offset: 0,
        min_data_length: 256,
        max_data_length: 32768,
        max_delta_counter: 10,
    };

    let mut sender = Profile8::new(config.clone());
    let mut receiver = Profile8::new(config);

    let mut group = c.benchmark_group("Profile8");

    for size in &[32, 64, 256, 1024] {
        let mut data = vec![0u8; *size];

        group.bench_with_input(BenchmarkId::new("protect", size), size, |b, &_size| {
            b.iter(|| {
                let mut data_copy = data.clone();
                sender.protect(black_box(&mut data_copy)).unwrap();
            })
        });

        // Prepare protected data for check benchmark
        sender.protect(&mut data).unwrap();

        group.bench_with_input(BenchmarkId::new("check", size), size, |b, &_size| {
            b.iter(|| {
                receiver.check(black_box(&data)).unwrap();
            })
        });
    }

    group.finish();
}

fn benchmark_profile11(c: &mut Criterion) {
    let config = Profile11Config {
        mode: Profile11IdMode::Nibble,
        max_delta_counter: 1,
        data_length: 40,
        ..Default::default()
    };

    let mut sender = Profile11::new(config.clone());
    let mut receiver = Profile11::new(config);

    let mut group = c.benchmark_group("Profile11");
    let mut data = vec![0x00, 0x00, 0x12, 0x34, 0x56]; // CRC, counter, user data

    group.bench_function("protect", |b| {
        b.iter(|| {
            let mut data_copy = data.clone();
            sender.protect(black_box(&mut data_copy)).unwrap();
        })
    });

    // Prepare protected data for check benchmark
    sender.protect(&mut data).unwrap();

    group.bench_function("check", |b| {
        b.iter(|| {
            receiver.check(black_box(&data)).unwrap();
        })
    });

    group.finish();
}

fn benchmark_profile22(c: &mut Criterion) {
    let config = Profile22Config {
        max_delta_counter: 1,
        data_length: 64,
        ..Default::default()
    };

    let mut sender = Profile22::new(config.clone());
    let mut receiver = Profile22::new(config);

    let mut group = c.benchmark_group("Profile22");
    let mut data = vec![0u8; 8]; // CRC, counter, user data

    group.bench_function("protect", |b| {
        b.iter(|| {
            let mut data_copy = data.clone();
            sender.protect(black_box(&mut data_copy)).unwrap();
        })
    });

    // Prepare protected data for check benchmark
    sender.protect(&mut data).unwrap();

    group.bench_function("check", |b| {
        b.iter(|| {
            receiver.check(black_box(&data)).unwrap();
        })
    });

    group.finish();
}

fn benchmark_all_profiles_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("ProfileComparison");

    // Profile 4 - 64 bytes
    let config4 = Profile4Config {
        data_id: 0x12345678,
        max_delta_counter: 1,
        min_data_length: 96,
        max_data_length: 32768,
        ..Default::default()
    };
    let mut profile4 = Profile4::new(config4);
    let mut data4 = vec![0u8; 64];

    group.bench_function("Profile4_64B_protect", |b| {
        b.iter(|| {
            let mut data_copy = data4.clone();
            profile4.protect(black_box(&mut data_copy)).unwrap();
        })
    });

    // Profile 5 - 8 bytes (fixed size)
    let config5 = Profile5Config {
        data_length: 8 * 8,
        data_id: 0x123,
        max_delta_counter: 1,
        offset: 0,
    };
    let mut profile5 = Profile5::new(config5);
    let mut data5 = vec![0u8; 8];

    group.bench_function("Profile5_8B_protect", |b| {
        b.iter(|| {
            let mut data_copy = data5.clone();
            profile5.protect(black_box(&mut data_copy)).unwrap();
        })
    });

    // Profile 7 - 64 bytes
    let config7 = Profile7Config {
        data_id: 0x0a0b0c0d,
        offset: 64,
        min_data_length: 20 * 8,
        max_data_length: 4096 * 8,
        max_delta_counter: 5,
    };
    let mut profile7 = Profile7::new(config7);
    let mut data7 = vec![0u8; 64];

    group.bench_function("Profile7_64B_protect", |b| {
        b.iter(|| {
            let mut data_copy = data7.clone();
            profile7.protect(black_box(&mut data_copy)).unwrap();
        })
    });

    // Profile 11 - 5 bytes
    let config11 = Profile11Config {
        mode: Profile11IdMode::Nibble,
        max_delta_counter: 1,
        data_length: 40,
        ..Default::default()
    };
    let mut profile11 = Profile11::new(config11);
    let mut data11 = vec![0x00, 0x00, 0x12, 0x34, 0x56];

    group.bench_function("Profile11_5B_protect", |b| {
        b.iter(|| {
            let mut data_copy = data11.clone();
            profile11.protect(black_box(&mut data_copy)).unwrap();
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_profile4,
    benchmark_profile5,
    benchmark_profile6,
    benchmark_profile7,
    benchmark_profile8,
    benchmark_profile11,
    benchmark_profile22,
    benchmark_all_profiles_comparison
);
criterion_main!(benches);