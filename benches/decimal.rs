use std::time::Duration;

use finfmt::asm::decimal::*;
use finfmt::primitive::decimal::MAX_DECIMAL_LEN;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

fn bench_format(suite: &mut Suite) {
    suite.group("format", |group| {
        quick(group);
        group.bench("format_u64_to_buf", |b| {
            b.iter(|| {
                let mut buf = [0u8; MAX_DECIMAL_LEN];
                let _ = format_u64_to_buf(&mut buf, black_box(1_234_567_890u64));
                black_box(buf)
            })
        });
        group.bench("format_i64_to_buf", |b| {
            b.iter(|| {
                let mut buf = [0u8; MAX_DECIMAL_LEN];
                let _ = format_i64_to_buf(&mut buf, black_box(-1_234_567_890i64));
                black_box(buf)
            })
        });
    });
}

fn bench_parse(suite: &mut Suite) {
    suite.group("parse", |group| {
        quick(group);
        group.bench("parse_u64_from_bytes", |b| {
            b.iter(|| black_box(parse_u64_from_bytes(black_box(b"1234567890"))))
        });
        group.bench("parse_usize_from_bytes", |b| {
            b.iter(|| black_box(parse_usize_from_bytes(black_box(b"1234567890"))))
        });
        group.bench("parse_i64_from_bytes", |b| {
            b.iter(|| black_box(parse_i64_from_bytes(black_box(b"-1234567890"))))
        });
    });
}

fn bench_sign(suite: &mut Suite) {
    suite.group("sign", |group| {
        quick(group);
        group.bench("decode_sign_cd", |b| {
            b.iter(|| {
                let mut input = black_box(&b"C"[..]);
                black_box(decode_sign_cd(&mut input))
            })
        });
        group.bench("encode_sign_cd", |b| {
            b.iter(|| {
                let mut buf = [0u8; 1];
                let mut out = &mut buf[..];
                let _ = encode_sign_cd(&mut out, black_box(true));
                black_box(buf)
            })
        });
        group.bench("encode_negative_prefix_minus", |b| {
            b.iter(|| {
                let mut buf = [0u8; 1];
                let mut out = &mut buf[..];
                let _ = encode_negative_prefix_minus(&mut out, black_box(true));
                black_box(buf)
            })
        });
        group.bench("decode_negative_prefix_minus", |b| {
            b.iter(|| {
                let mut input = black_box(&b"-"[..]);
                black_box(decode_negative_prefix_minus(&mut input))
            })
        });
        group.bench("prepend_minus_to_digits", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = prepend_minus_to_digits(&mut out, black_box(b"1234567"));
                black_box(buf)
            })
        });
    });
}

fn bench_implied(suite: &mut Suite) {
    suite.group("implied", |group| {
        quick(group);
        group.bench("encode_decimal_implied_scale2_signed", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = encode_decimal_implied_scale2_signed(&mut out, black_box(b"-123.45"));
                black_box(buf)
            })
        });
        group.bench("decode_decimal_implied_scale2", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = decode_decimal_implied_scale2(&mut out, black_box(b"12345"));
                black_box(buf)
            })
        });
    });
}

fn bench_overpunch_packed_sign(suite: &mut Suite) {
    suite.group("overpunch_packed_sign", |group| {
        quick(group);
        group.bench("encode_overpunch_digit_for", |b| {
            b.iter(|| black_box(encode_overpunch_digit_for(black_box(true), black_box(7))))
        });
        group.bench("decode_overpunch_digit_byte", |b| {
            b.iter(|| black_box(decode_overpunch_digit_byte(black_box(0xD7))))
        });
        group.bench("encode_packed_sign_signed", |b| {
            b.iter(|| black_box(encode_packed_sign_signed(black_box(true))))
        });
        group.bench("encode_packed_sign_unsigned", |b| {
            b.iter(|| black_box(encode_packed_sign_unsigned(black_box(false))))
        });
        group.bench("decode_packed_sign_nibble", |b| {
            b.iter(|| black_box(decode_packed_sign_nibble(black_box(0x0D))))
        });
        group.bench("packed_decimal_max_digits_8", |b| {
            b.iter(|| black_box(packed_decimal_max_digits_8(black_box(8))))
        });
    });
}

fn bench_fixed_ascii(suite: &mut Suite) {
    suite.group("fixed_ascii", |group| {
        quick(group);
        group.bench("encode_decimal_ascii_fixed_2", |b| {
            b.iter(|| {
                let mut buf = [0u8; 2];
                let mut out = &mut buf[..];
                let _ = encode_decimal_ascii_fixed_2(&mut out, black_box(42));
                black_box(buf)
            })
        });
        group.bench("decode_decimal_ascii_fixed_2", |b| {
            b.iter(|| {
                let mut input = black_box(&b"42"[..]);
                black_box(decode_decimal_ascii_fixed_2(&mut input))
            })
        });
    });
}

fn bench_fixed_ebcdic(suite: &mut Suite) {
    suite.group("fixed_ebcdic", |group| {
        quick(group);
        group.bench("encode_decimal_ebcdic_fixed_2", |b| {
            b.iter(|| {
                let mut buf = [0u8; 2];
                let mut out = &mut buf[..];
                let _ = encode_decimal_ebcdic_fixed_2(&mut out, black_box(42));
                black_box(buf)
            })
        });
        group.bench("encode_decimal_ebcdic_blankable_fixed_2", |b| {
            b.iter(|| {
                let mut buf = [0u8; 2];
                let mut out = &mut buf[..];
                let _ = encode_decimal_ebcdic_blankable_fixed_2(&mut out, black_box(0));
                black_box(buf)
            })
        });
        group.bench("decode_decimal_ebcdic_fixed_2", |b| {
            b.iter(|| {
                let mut input = black_box(&[0xF4u8, 0xF2][..]);
                black_box(decode_decimal_ebcdic_fixed_2(&mut input))
            })
        });
        group.bench("decode_decimal_ebcdic_blankable_fixed_2", |b| {
            b.iter(|| {
                let mut input = black_box(&[0x40u8, 0x40][..]);
                black_box(decode_decimal_ebcdic_blankable_fixed_2(&mut input))
            })
        });
    });
}

fn bench_signed_zoned(suite: &mut Suite) {
    suite.group("signed_zoned", |group| {
        quick(group);
        group.bench("encode_decimal_ebcdic_signed_fixed_8", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = encode_decimal_ebcdic_signed_fixed_8(&mut out, black_box(b"-1234567"));
                black_box(buf)
            })
        });
        group.bench("decode_decimal_ebcdic_signed_fixed_8", |b| {
            b.iter(|| {
                let mut input = black_box(&[0xF0u8, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xD7][..]);
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = decode_decimal_ebcdic_signed_fixed_8(&mut input, &mut out);
                black_box(buf)
            })
        });
    });
}

fn bench_packed(suite: &mut Suite) {
    suite.group("packed", |group| {
        quick(group);
        group.bench("encode_decimal_packed_fixed_8", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = encode_decimal_packed_fixed_8(&mut out, black_box(b"1234567890123"));
                black_box(buf)
            })
        });
        group.bench("encode_decimal_packed_signed_fixed_8", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = encode_decimal_packed_signed_fixed_8(&mut out, black_box(b"-1234567890123"));
                black_box(buf)
            })
        });
        group.bench("decode_decimal_packed_fixed_8", |b| {
            b.iter(|| {
                let mut input = black_box(&[0x12u8, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x5F][..]);
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = decode_decimal_packed_fixed_8(&mut input, &mut out);
                black_box(buf)
            })
        });
        group.bench("decode_decimal_packed_signed_fixed_8", |b| {
            b.iter(|| {
                let mut input = black_box(&[0x12u8, 0x34, 0x56, 0x78, 0x90, 0x12, 0x34, 0x5D][..]);
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = decode_decimal_packed_signed_fixed_8(&mut input, &mut out);
                black_box(buf)
            })
        });
    });
}

zenbench::main!(
    bench_format,
    bench_parse,
    bench_sign,
    bench_implied,
    bench_overpunch_packed_sign,
    bench_fixed_ascii,
    bench_fixed_ebcdic,
    bench_signed_zoned,
    bench_packed,
);
