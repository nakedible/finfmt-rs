use std::time::Duration;

use finfmt::asm::nibble::*;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

const DIGITS_8: &[u8] = b"12345678";
const HEX_8: &[u8] = b"1A2B3C4D";
const PACKED_4: &[u8] = b"\x12\x34\x56\x78";

fn bench_pack(suite: &mut Suite) {
    suite.group("pack", |group| {
        quick(group);
        group.bench("pack_nibbles_bcdz_left", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = pack_nibbles_bcdz_left(&mut out, black_box(DIGITS_8));
                black_box(buf)
            })
        });
        group.bench("pack_nibbles_bcdz_right", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = pack_nibbles_bcdz_right(&mut out, black_box(DIGITS_8));
                black_box(buf)
            })
        });
        group.bench("pack_expanded_nibbles_hex_upper", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = pack_expanded_nibbles_hex_upper(&mut out, black_box(HEX_8));
                black_box(buf)
            })
        });
    });
}

fn bench_unpack(suite: &mut Suite) {
    suite.group("unpack", |group| {
        quick(group);
        group.bench("unpack_nibbles_bcdz", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = unpack_nibbles_bcdz(&mut out, black_box(PACKED_4));
                black_box(buf)
            })
        });
        group.bench("unpack_padded_nibbles_bcdz_right", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = unpack_padded_nibbles_bcdz_right(&mut out, black_box(PACKED_4), black_box(8));
                black_box(buf)
            })
        });
        group.bench("unpack_padded_nibbles_bcdz_left", |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let _ = unpack_padded_nibbles_bcdz_left(&mut out, black_box(PACKED_4), black_box(8));
                black_box(buf)
            })
        });
    });
}

fn bench_validate(suite: &mut Suite) {
    suite.group("validate", |group| {
        quick(group);
        group.bench("validate_nibbles_hex_upper", |b| {
            b.iter(|| black_box(validate_nibbles_hex_upper(black_box(HEX_8))))
        });
    });
}

zenbench::main!(bench_pack, bench_unpack, bench_validate);
