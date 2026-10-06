use std::time::Duration;

use finfmt::asm::codepage::*;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

const ASCII_INPUT: &[u8] = b"Hello, World 1234567890!";
const CP037_INPUT: &[u8] = &[
    0xC8, 0x85, 0x93, 0x93, 0x96, 0x6B, 0x40, 0xE6, 0x96, 0x99, 0x93, 0x84, 0x40, 0xF1, 0xF2, 0xF3, 0xF4, 0xF5, 0xF6, 0xF7, 0xF8, 0xF9,
    0xF0, 0x5A,
];
const IDENTITY: [u8; 256] = {
    let mut table = [0; 256];
    let mut i = 0;
    while i < 256 {
        table[i] = i as u8;
        i += 1;
    }
    table
};

fn bench_ascii_subset(suite: &mut Suite) {
    suite.group("ascii_subset", |group| {
        quick(group);
        group.bench("translate_bytes_table", |b| {
            b.iter(|| {
                let mut buf = [0u8; 24];
                let _ = translate_bytes_table(&mut buf, black_box(ASCII_INPUT), black_box(&IDENTITY));
                black_box(buf)
            })
        });
        group.bench("translate_bytes_inplace_table", |b| {
            b.iter(|| {
                let mut buf = *b"Hello, World 1234567890!";
                translate_bytes_inplace_table(black_box(&mut buf), black_box(&IDENTITY));
                black_box(buf)
            })
        });
        group.bench("encode_ascii_subset_037", |b| {
            b.iter(|| {
                let mut buf = *b"Hello, World 1234567890!";
                encode_ascii_subset_037(black_box(&mut buf));
                black_box(buf)
            })
        });
        group.bench("decode_ascii_subset_037", |b| {
            b.iter(|| {
                let mut buf = [0u8; 24];
                let _ = decode_ascii_subset_037(&mut buf, black_box(CP037_INPUT));
                black_box(buf)
            })
        });
    });
}

fn bench_text(suite: &mut Suite) {
    suite.group("text", |group| {
        quick(group);
        for (name, ch) in [("encode_char_1142_latin1", 'Æ'), ("encode_char_1142_euro", '€')] {
            group.bench(name, move |b| b.iter(|| black_box(encode_char_1142(black_box(ch)))));
        }
        group.bench("encode_char_850_box", |b| b.iter(|| black_box(encode_char_850(black_box('┼')))));
        for (name, input) in [
            ("encode_text_1142_ascii", ASCII_INPUT),
            ("encode_text_1142_mixed", "ABCÆØÅæøå€".as_bytes()),
        ] {
            group.bench(name, |b| {
                b.iter(|| {
                    let mut buf = [0u8; 32];
                    let _ = encode_text_1142(&mut &mut buf[..], black_box(input));
                    black_box(buf)
                })
            });
        }
        for (name, input) in [
            ("decode_text_1142_ascii", &b"\xC1\xC2\xC3\xF1\xF2\xF3"[..]),
            ("decode_text_1142_mixed", b"\xC1\xC2\xC3\x7B\x7C\x5B\xC0\x6A\xD0\x5A"),
        ] {
            group.bench(name, |b| {
                b.iter(|| {
                    let mut buf = [0u8; 32];
                    let _ = decode_text_1142(&mut &mut buf[..], black_box(input));
                    black_box(buf)
                })
            });
        }
        group.bench("encode_text_latin1_ascii", |b| {
            b.iter(|| {
                let mut buf = [0u8; 32];
                let _ = encode_text_latin1(&mut &mut buf[..], black_box(ASCII_INPUT));
                black_box(buf)
            })
        });
        group.bench("decode_text_latin1_ascii", |b| {
            b.iter(|| {
                let mut buf = [0u8; 32];
                let _ = decode_text_latin1(&mut &mut buf[..], black_box(ASCII_INPUT));
                black_box(buf)
            })
        });
        for (name, input) in [
            ("encode_charset_1142_field_ascii", ASCII_INPUT),
            ("encode_charset_1142_field_mixed", "ABCÆØÅæøå€".as_bytes()),
            ("encode_charset_1142_field_euro", "€€€€€€€€€€".as_bytes()),
        ] {
            group.bench(name, |b| {
                b.iter(|| {
                    let mut buf = [0u8; 32];
                    let _ = black_box(encode_charset_1142_field(&mut &mut buf[..], black_box(input)));
                    black_box(buf)
                })
            });
        }
    });
}

zenbench::main!(bench_ascii_subset, bench_text);
