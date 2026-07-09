use std::time::Duration;

use finfmt::asm::int::*;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

fn bench_be_bytes(suite: &mut Suite) {
    suite.group("be_bytes", |group| {
        quick(group);
        group.bench("encode_be_bytes_4_zero", |b| {
            b.iter(|| {
                let mut buf = [0u8; 4];
                let mut out = &mut buf[..];
                let _ = encode_be_bytes_4_zero(&mut out, black_box(&[0x12, 0x34, 0x56, 0x78][..]));
                black_box(buf)
            })
        });
        group.bench("extend_be_bytes_4_zero_to_8", |b| {
            b.iter(|| {
                let mut input = black_box(&[0x12u8, 0x34, 0x56, 0x78][..]);
                black_box(extend_be_bytes_4_zero_to_8(&mut input))
            })
        });
    });
}

fn bench_nibble_int(suite: &mut Suite) {
    suite.group("nibble_int", |group| {
        quick(group);
        group.bench("validate_nibble_int_fixed_hex_upper_4", |b| {
            b.iter(|| black_box(validate_nibble_int_fixed_hex_upper_4(black_box(b"ABCD"))))
        });
        group.bench("encode_nibble_int_fixed_hex_upper_4", |b| {
            b.iter(|| {
                let mut buf = [0u8; 4];
                let mut out = &mut buf[..];
                let _ = encode_nibble_int_fixed_hex_upper_4(&mut out, black_box(0xABCD));
                black_box(buf)
            })
        });
        group.bench("decode_nibble_int_fixed_hex_upper_4", |b| {
            b.iter(|| {
                let mut input = black_box(&b"ABCD"[..]);
                black_box(decode_nibble_int_fixed_hex_upper_4(&mut input))
            })
        });
    });
}

fn bench_binary_be(suite: &mut Suite) {
    suite.group("binary_be", |group| {
        quick(group);
        group.bench("encode_binary_u64_be_fixed_4", |b| {
            b.iter(|| {
                let mut buf = [0u8; 4];
                let mut out = &mut buf[..];
                let _ = encode_binary_u64_be_fixed_4(&mut out, black_box(0x1234_5678));
                black_box(buf)
            })
        });
        group.bench("decode_binary_u64_be_fixed_4", |b| {
            b.iter(|| {
                let mut input = black_box(&[0x12u8, 0x34, 0x56, 0x78][..]);
                black_box(decode_binary_u64_be_fixed_4(&mut input))
            })
        });
        group.bench("validate_binary_i64_be_fixed_4", |b| {
            b.iter(|| black_box(validate_binary_i64_be_fixed_4(black_box(-1_000_000))))
        });
        group.bench("encode_binary_i64_be_fixed_4", |b| {
            b.iter(|| {
                let mut buf = [0u8; 4];
                let mut out = &mut buf[..];
                let _ = encode_binary_i64_be_fixed_4(&mut out, black_box(-1_000_000));
                black_box(buf)
            })
        });
        group.bench("decode_binary_i64_be_fixed_4", |b| {
            b.iter(|| {
                let mut input = black_box(&[0xFFu8, 0xF0, 0xBD, 0xC0][..]);
                black_box(decode_binary_i64_be_fixed_4(&mut input))
            })
        });
        group.bench("decode_signed_magnitude_i64_combine", |b| {
            b.iter(|| black_box(decode_signed_magnitude_i64_combine(black_box(true), black_box(1_000_000))))
        });
    });
}

zenbench::main!(bench_be_bytes, bench_nibble_int, bench_binary_be);
