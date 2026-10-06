use std::time::Duration;

use finfmt::asm::bytes::*;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

const INPUT_8: &[u8] = b"12345678";
const FILLED_8_SPACE: &[u8] = &[0x40; 8];
const REPEATED_BLOCK: &[u8] = b"AB";
const DELIMITED: &[u8] = b"abcd|wxyz";

fn bench_bytes(suite: &mut Suite) {
    suite.group("bytes", |group| {
        quick(group);

        group.bench("copy_bytes_through", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = copy_bytes_through(&mut out, black_box(INPUT_8));
                black_box(buf)
            })
        });

        group.bench("take_bytes_8", |b| {
            b.iter(|| {
                let mut input = black_box(INPUT_8);
                black_box(take_bytes_8(&mut input))
            })
        });

        group.bench("reserve_bytes_8", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = black_box(reserve_bytes_8(&mut out)).map(|area| area.fill(1));
                black_box(buf)
            })
        });

        group.bench("reserve_filled_8_ebcdic_space", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = reserve_filled_8_ebcdic_space(&mut out);
                black_box(buf)
            })
        });

        group.bench("is_filled_ebcdic_space", |b| {
            b.iter(|| black_box(is_filled_ebcdic_space(black_box(FILLED_8_SPACE))))
        });

        group.bench("take_delimited_pipe", |b| {
            b.iter(|| {
                let mut input = black_box(DELIMITED);
                black_box(take_delimited_pipe(&mut input))
            })
        });
    });

    suite.group("repeated_block", |group| {
        quick(group);

        group.bench("fill_repeated_runtime", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let _ = fill_repeated_runtime(&mut buf, black_box(REPEATED_BLOCK));
                black_box(buf)
            })
        });

        group.bench("fill_repeated_4", |b| {
            b.iter(|| {
                let mut buf = [0u8; 64];
                let _ = fill_repeated_4(&mut buf, black_box(b"ABCD"));
                black_box(buf)
            })
        });
    });
}

zenbench::main!(bench_bytes);
