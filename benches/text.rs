use std::time::Duration;

use finfmt::asm::text::*;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

const SHORT_BYTES: &[u8] = b"Hi";
const PADDED_RIGHT: &[u8; 8] = b"Hi      ";
const PADDED_LEFT: &[u8; 8] = b"      Hi";

fn bench_encode_bytes(suite: &mut Suite) {
    suite.group("encode_padded", |group| {
        quick(group);
        group.bench("encode_padded_right_8_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = encode_padded_right_8_space(&mut out, black_box(SHORT_BYTES));
                black_box(buf)
            })
        });
        group.bench("encode_padded_left_8_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = encode_padded_left_8_space(&mut out, black_box(SHORT_BYTES));
                black_box(buf)
            })
        });
        group.bench("encode_padded_fixed_8_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = encode_padded_fixed_8_space(&mut out, black_box(SHORT_BYTES));
                black_box(buf)
            })
        });
        group.bench("encode_padded_no_padding", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let _ = encode_padded_right_8_space(&mut buf.as_mut_slice(), black_box(b"123456789ABC"));
                black_box(buf)
            })
        });
        group.bench("truncate_bytes_left_8", |b| {
            b.iter(|| black_box(truncate_bytes_left_8(black_box(b"123456789ABC"))))
        });
        group.bench("truncate_bytes_right_8", |b| {
            b.iter(|| black_box(truncate_bytes_right_8(black_box(b"123456789ABC"))))
        });
    });
}

fn bench_decode_bytes(suite: &mut Suite) {
    suite.group("decode_padded", |group| {
        quick(group);
        group.bench("decode_padded_right_8_space", |b| {
            b.iter(|| {
                let input = black_box(PADDED_RIGHT);
                black_box(decode_padded_right_8_space(input))
            })
        });
        group.bench("decode_padded_left_8_space", |b| {
            b.iter(|| {
                let input = black_box(PADDED_LEFT);
                black_box(decode_padded_left_8_space(input))
            })
        });
        group.bench("decode_padded_even_right_question", |b| {
            b.iter(|| {
                black_box(decode_padded_even_right_question(
                    black_box(b"1234567890123456789?"),
                    black_box(Some(19)),
                ))
            })
        });
        group.bench("decode_padded_exact_right_8_ebcdic_space", |b| {
            b.iter(|| {
                black_box(decode_padded_exact_right_8_ebcdic_space(
                    black_box(b"abcd\x40\x40\x40\x40"),
                    black_box(4),
                ))
            })
        });
        group.bench("decode_padded_protected_6", |b| {
            b.iter(|| black_box(decode_padded_protected_6(black_box(b"        "))))
        });
    });
}

fn bench_truncate_str(suite: &mut Suite) {
    suite.group("truncate_str", |group| {
        quick(group);
        group.bench("truncate_str_left_8_ascii", |b| {
            b.iter(|| black_box(truncate_str_left_8(black_box("123456789ABC"))))
        });
        group.bench("truncate_str_right_8_ascii", |b| {
            b.iter(|| black_box(truncate_str_right_8(black_box("123456789ABC"))))
        });
        group.bench("truncate_str_left_8_latin", |b| {
            b.iter(|| black_box(truncate_str_left_8(black_box("ÆØÅæøåÆØÅæøå"))))
        });
        group.bench("encode_truncated_ascii_4", |b| {
            b.iter(|| {
                let mut buf = [0u8; 4];
                let _ = encode_truncated_ascii_4(&mut buf.as_mut_slice(), black_box("ABCDE"));
                black_box(buf)
            })
        });
    });
}

zenbench::main!(bench_encode_bytes, bench_decode_bytes, bench_truncate_str);
