use std::time::Duration;

use finfmt::asm::bertlv::*;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

const TAG_2BYTE: &[u8] = b"\x9F\x02";
const LEN_LONG2: &[u8] = b"\x82\x01\x00";
const TLV_ENTRY: &[u8] = b"\x9F\x02\x06\xAB\xCD\xEF\x12\x34\x56";

fn bench_bertlv(suite: &mut Suite) {
    suite.group("bertlv", |group| {
        quick(group);
        group.bench("encode_bertag_runtime", |b| {
            b.iter(|| {
                let mut buf = [0u8; 8];
                let mut out = &mut buf[..];
                let _ = encode_bertag_runtime(&mut out, black_box(TAG_2BYTE));
                black_box(buf)
            })
        });
        group.bench("decode_bertag_runtime", |b| {
            b.iter(|| {
                let mut input = black_box(TAG_2BYTE);
                black_box(decode_bertag_runtime(&mut input))
            })
        });
        group.bench("encode_berlen_runtime", |b| {
            b.iter(|| {
                let mut buf = [0u8; 4];
                let mut out = &mut buf[..];
                let _ = encode_berlen_runtime(&mut out, black_box(0x100));
                black_box(buf)
            })
        });
        group.bench("decode_berlen_runtime", |b| {
            b.iter(|| {
                let mut input = black_box(LEN_LONG2);
                black_box(decode_berlen_runtime(&mut input))
            })
        });
        group.bench("encoded_berlen_runtime", |b| {
            b.iter(|| black_box(encoded_berlen_runtime(black_box(0x100))))
        });
        group.bench("parse_hex_tag_runtime", |b| {
            b.iter(|| black_box(parse_hex_tag_runtime(black_box("9F02"))))
        });
        group.bench("tag_eq_hex_9f02", |b| b.iter(|| black_box(tag_eq_hex_9f02(black_box(TAG_2BYTE)))));
        group.bench("decode_ber_tlv_entry_runtime", |b| {
            b.iter(|| {
                let mut input = black_box(TLV_ENTRY);
                black_box(decode_ber_tlv_entry_runtime(&mut input))
            })
        });
    });
}

zenbench::main!(bench_bertlv);
