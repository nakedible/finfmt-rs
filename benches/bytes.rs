use std::time::Duration;

use finfmt::asm::bytes::*;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

const INPUT_8: &[u8] = b"12345678";
const FILLED_8_SPACE: &[u8] = &[0x40; 8];
const REPEATED_BLOCK: &[u8] = b"AB";
const REPEATED_INPUT: &[u8] = b"ABABABAB";
const DELIMITED: &[u8] = b"abcd|wxyz";

fn bench_bytes(suite: &mut Suite) {
    suite.group("bytes", |group| {
        quick(group);

        group.bench("validate_exact_length_8", |b| {
            b.iter(|| validate_exact_length_8(black_box(INPUT_8)))
        });

        group.bench("copy_bytes_through", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = copy_bytes_through(&mut out, black_box(INPUT_8));
                black_box(buf)
            })
        });

        group.bench("encode_exact_bytes_8", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = encode_exact_bytes_8(&mut out, black_box(INPUT_8));
                black_box(buf)
            })
        });

        group.bench("decode_exact_bytes_8", |b| {
            b.iter(|| {
                let mut input = black_box(INPUT_8);
                black_box(decode_exact_bytes_8(&mut input))
            })
        });

        group.bench("reserve_filled_area_8_ebcdic_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = reserve_filled_area_8_ebcdic_space(&mut out);
                black_box(buf)
            })
        });

        group.bench("decode_filled_prefix_8_ebcdic_space", |b| {
            let input = b"abcd\x40\x40\x40\x40";
            b.iter(|| {
                let mut input = black_box(&input[..]);
                black_box(decode_filled_prefix_8_ebcdic_space(&mut input, 4))
            })
        });

        group.bench("fill_tail_ebcdic_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let _ = fill_tail_ebcdic_space(&mut buf, black_box(2));
                black_box(buf)
            })
        });

        group.bench("all_bytes_eq_ebcdic_space", |b| {
            b.iter(|| black_box(all_bytes_eq_ebcdic_space(black_box(FILLED_8_SPACE))))
        });

        group.bench("validate_all_bytes_ebcdic_space", |b| {
            b.iter(|| black_box(validate_all_bytes_ebcdic_space(black_box(FILLED_8_SPACE))))
        });

        group.bench("fill_repeated_block_runtime", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let _ = fill_repeated_block_runtime(&mut buf, black_box(2), black_box(REPEATED_BLOCK));
                black_box(buf)
            })
        });

        group.bench("validate_repeating_block_runtime", |b| {
            b.iter(|| {
                black_box(validate_repeating_block_runtime(
                    black_box(REPEATED_INPUT),
                    black_box(REPEATED_BLOCK),
                ))
            })
        });

        group.bench("contains_byte_pipe", |b| {
            b.iter(|| black_box(contains_byte_pipe(black_box(DELIMITED))))
        });

        group.bench("split_delimited_bytes_pipe", |b| {
            b.iter(|| {
                let mut input = black_box(DELIMITED);
                black_box(split_delimited_bytes_pipe(&mut input))
            })
        });
    });
}

zenbench::main!(bench_bytes);
