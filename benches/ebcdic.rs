use std::time::Duration;

use finfmt::asm::ebcdic::*;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

const ASCII_INPUT: &[u8] = b"Hello, World 1234567890!";
const EBCDIC_INPUT: &[u8] = &[
    0xC8, 0x85, 0x93, 0x93, 0x96, 0x6B, 0x40, 0xE6, 0x96, 0x99, 0x93, 0x84, 0x40, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9,
    0xF0, 0x4F,
];
const UTF8_1142: &str = "ABCÆØÅæøå";

fn bench_translate(suite: &mut Suite) {
    suite.group("translate", |group| {
        quick(group);
        group.bench("translate_bytes_ascii_to_037", |b| {
            b.iter(|| {
                let mut buf = [0u8; 24];
                translate_bytes_ascii_to_037(&mut buf, black_box(ASCII_INPUT));
                black_box(buf)
            })
        });
        group.bench("translate_bytes_037_to_ascii", |b| {
            b.iter(|| {
                let mut buf = [0u8; 24];
                translate_bytes_037_to_ascii(&mut buf, black_box(EBCDIC_INPUT));
                black_box(buf)
            })
        });
        group.bench("translate_bytes_inplace_ascii_to_037", |b| {
            b.iter(|| {
                let mut buf = *b"Hello, World 1234567890!";
                translate_bytes_inplace_ascii_to_037(black_box(&mut buf));
                black_box(buf)
            })
        });
        group.bench("translate_bytes_inplace_037_to_ascii", |b| {
            b.iter(|| {
                let mut buf = *b"\xC8\x85\x93\x93\x96\x6B\x40\xE6\x96\x99\x93\x84\x40\xF1\xF2\xF3\xF4\xF5\xF6\xF7\xF8\xF9\xF0\x4F";
                translate_bytes_inplace_037_to_ascii(black_box(&mut buf));
                black_box(buf)
            })
        });
    });
}

fn bench_1142(suite: &mut Suite) {
    suite.group("ebcdic_1142", |group| {
        quick(group);
        group.bench("encode_ebcdic_1142_char_lookup", |b| {
            b.iter(|| black_box(encode_ebcdic_1142_char_lookup(black_box('Æ'))))
        });
        group.bench("utf8_to_ebcdic_1142_runtime", |b| {
            b.iter(|| {
                let mut buf = [0u8; 32];
                let mut out = &mut buf[..];
                let _ = utf8_to_ebcdic_1142_runtime(&mut out, black_box(UTF8_1142.as_bytes()));
                black_box(buf)
            })
        });
        group.bench("ebcdic_1142_to_utf8_runtime", |b| {
            let input = b"\xC1\xC2\xC3\x7B\x7C\x5B\xC0\x6A\xD0";
            b.iter(|| {
                let mut buf = [0u8; 32];
                let mut out = &mut buf[..];
                let _ = ebcdic_1142_to_utf8_runtime(&mut out, black_box(input));
                black_box(buf)
            })
        });
    });
}

zenbench::main!(bench_translate, bench_1142);
