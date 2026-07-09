use std::time::Duration;

use finfmt::asm::text::*;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

const SHORT_BYTES: &[u8] = b"Hi";
const SHORT_STR: &str = "Hi";
const PADDED_RIGHT: &[u8] = b"Hi      ";
const PADDED_LEFT: &[u8] = b"      Hi";

fn bench_encode_bytes(suite: &mut Suite) {
    suite.group("encode_bytes", |group| {
        quick(group);
        group.bench("encode_bytes_pad_right_8_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = encode_bytes_pad_right_8_space(&mut out, black_box(SHORT_BYTES));
                black_box(buf)
            })
        });
        group.bench("encode_bytes_pad_left_8_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = encode_bytes_pad_left_8_space(&mut out, black_box(SHORT_BYTES));
                black_box(buf)
            })
        });
        group.bench("encode_bytes_fixed_8_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = encode_bytes_fixed_8_space(&mut out, black_box(SHORT_BYTES));
                black_box(buf)
            })
        });
    });
}

fn bench_decode_bytes(suite: &mut Suite) {
    suite.group("decode_bytes", |group| {
        quick(group);
        group.bench("decode_bytes_strip_right_8_space", |b| {
            b.iter(|| {
                let mut input = black_box(PADDED_RIGHT);
                black_box(decode_bytes_strip_right_8_space(&mut input))
            })
        });
        group.bench("decode_bytes_strip_left_8_space", |b| {
            b.iter(|| {
                let mut input = black_box(PADDED_LEFT);
                black_box(decode_bytes_strip_left_8_space(&mut input))
            })
        });
    });
}

fn bench_encode_ascii(suite: &mut Suite) {
    suite.group("encode_ascii", |group| {
        quick(group);
        group.bench("encode_ascii_pad_right_8_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = encode_ascii_pad_right_8_space(&mut out, black_box(SHORT_STR));
                black_box(buf)
            })
        });
        group.bench("encode_ascii_pad_left_8_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = encode_ascii_pad_left_8_space(&mut out, black_box(SHORT_STR));
                black_box(buf)
            })
        });
    });
}

fn bench_decode_ascii(suite: &mut Suite) {
    suite.group("decode_ascii", |group| {
        quick(group);
        group.bench("decode_ascii_strip_right_8_space", |b| {
            b.iter(|| {
                let mut input = black_box(PADDED_RIGHT);
                black_box(decode_ascii_strip_right_8_space(&mut input))
            })
        });
        group.bench("decode_ascii_strip_left_8_space", |b| {
            b.iter(|| {
                let mut input = black_box(PADDED_LEFT);
                black_box(decode_ascii_strip_left_8_space(&mut input))
            })
        });
    });
}

zenbench::main!(bench_encode_bytes, bench_decode_bytes, bench_encode_ascii, bench_decode_ascii);
