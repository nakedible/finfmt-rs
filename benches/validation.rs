use std::time::Duration;

use finfmt::asm::validation::*;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

const NUMERIC: &[u8] = b"1234567890123456";
const ALPHA: &[u8] = b"AbcDefGhIj";
const ALPHANUM: &[u8] = b"AB12cd34EF56";
const ASCII_PRINT: &[u8] = b"Hello, World 123!";
const UPPER_ALPHA: &[u8] = b"ABCDEFGH";
const UPPER_ALPHANUM: &[u8] = b"AB12CD34";
const HEX_LOWER: &[u8] = b"abc123def4";
const HEX_UPPER: &[u8] = b"ABC123DEF4";
const HEX_MIXED: &[u8] = b"abc123DEF4";
const BCDZ: &[u8] = b"123:456;789";
const TRACK2: &[u8] = b"1234567890=2601";
const BCD_BYTES: &[u8] = b"\x12\x34\x56\x78";
const BINARY: &[u8] = b"\x00\xFF\x42\x99";
const ISO_8859_1_STR: &str = "héllo";
const EBCDIC_PRINT: &[u8] = &[0xC8, 0x85, 0x93, 0x93, 0x96];
const SIGNED_DECIMAL: &[u8] = b"-1234567";

fn bench_class_predicates(suite: &mut Suite) {
    suite.group("class_predicates", |group| {
        quick(group);
        group.bench("validate_numeric_1_19", |b| {
            b.iter(|| black_box(validate_numeric_1_19(black_box(NUMERIC))))
        });
        group.bench("validate_alpha_1_99", |b| {
            b.iter(|| black_box(validate_alpha_1_99(black_box(ALPHA))))
        });
        group.bench("validate_alphanum_1_99", |b| {
            b.iter(|| black_box(validate_alphanum_1_99(black_box(ALPHANUM))))
        });
        group.bench("validate_ascii_1_99", |b| {
            b.iter(|| black_box(validate_ascii_1_99(black_box(ASCII_PRINT))))
        });
        group.bench("validate_ascii_printable_1_99", |b| {
            b.iter(|| black_box(validate_ascii_printable_1_99(black_box(ASCII_PRINT))))
        });
        group.bench("validate_upper_alpha_1_99", |b| {
            b.iter(|| black_box(validate_upper_alpha_1_99(black_box(UPPER_ALPHA))))
        });
        group.bench("validate_upper_alphanum_1_99", |b| {
            b.iter(|| black_box(validate_upper_alphanum_1_99(black_box(UPPER_ALPHANUM))))
        });
        group.bench("validate_upper_ascii_printable_1_99", |b| {
            b.iter(|| black_box(validate_upper_ascii_printable_1_99(black_box(b"HELLO 123!"))))
        });
    });
}

fn bench_hex(suite: &mut Suite) {
    suite.group("hex", |group| {
        quick(group);
        group.bench("validate_hex_1_99", |b| {
            b.iter(|| black_box(validate_hex_1_99(black_box(HEX_MIXED))))
        });
        group.bench("validate_hex_upper_1_99", |b| {
            b.iter(|| black_box(validate_hex_upper_1_99(black_box(HEX_UPPER))))
        });
        group.bench("validate_hex_lower_1_99", |b| {
            b.iter(|| black_box(validate_hex_lower_1_99(black_box(HEX_LOWER))))
        });
        group.bench("validate_hex_even_2_98", |b| {
            b.iter(|| black_box(validate_hex_even_2_98(black_box(HEX_MIXED))))
        });
        group.bench("validate_hex_upper_even_2_98", |b| {
            b.iter(|| black_box(validate_hex_upper_even_2_98(black_box(HEX_UPPER))))
        });
        group.bench("validate_hex_lower_even_2_98", |b| {
            b.iter(|| black_box(validate_hex_lower_even_2_98(black_box(HEX_LOWER))))
        });
    });
}

fn bench_specialty(suite: &mut Suite) {
    suite.group("specialty", |group| {
        quick(group);
        group.bench("validate_bcdz_1_99", |b| b.iter(|| black_box(validate_bcdz_1_99(black_box(BCDZ)))));
        group.bench("validate_track2_1_37", |b| {
            b.iter(|| black_box(validate_track2_1_37(black_box(TRACK2))))
        });
        group.bench("validate_bcd_bytes_1_10", |b| {
            b.iter(|| black_box(validate_bcd_bytes_1_10(black_box(BCD_BYTES))))
        });
        group.bench("validate_binary_1_99", |b| {
            b.iter(|| black_box(validate_binary_1_99(black_box(BINARY))))
        });
        group.bench("validate_iso8859_1_str_1_99", |b| {
            b.iter(|| black_box(validate_iso8859_1_str_1_99(black_box(ISO_8859_1_STR))))
        });
        group.bench("validate_ebcdic_1142_text_1_99", |b| {
            b.iter(|| black_box(validate_ebcdic_1142_text_1_99(black_box(EBCDIC_PRINT))))
        });
        group.bench("validate_ebcdic_printable_1_99", |b| {
            b.iter(|| black_box(validate_ebcdic_printable_1_99(black_box(EBCDIC_PRINT))))
        });
    });
}

fn bench_signed_and_implied(suite: &mut Suite) {
    suite.group("signed_and_implied", |group| {
        quick(group);
        group.bench("split_signed_input_runtime", |b| {
            b.iter(|| black_box(split_signed_input_runtime(black_box(SIGNED_DECIMAL))))
        });
        group.bench("parse_signed_decimal_19", |b| {
            b.iter(|| black_box(parse_signed_decimal_19(black_box(SIGNED_DECIMAL))))
        });
        group.bench("validate_decimal_implied_scale2_signed", |b| {
            b.iter(|| black_box(validate_decimal_implied_scale2_signed(black_box(b"-123.45"))))
        });
    });
}

zenbench::main!(bench_class_predicates, bench_hex, bench_specialty, bench_signed_and_implied);
