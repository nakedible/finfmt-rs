use std::time::Duration;

use finfmt::asm::bitmap::*;
use finfmt::primitive::bitmap::Bitmap;
use zenbench::prelude::*;

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

fn sample_bitmap() -> Bitmap {
    let mut bitmap = Bitmap::new();
    for &id in &[
        2u16, 3, 4, 7, 11, 12, 13, 14, 18, 22, 32, 35, 37, 41, 42, 43, 49, 52, 64, 73, 96, 102, 128,
    ] {
        bitmap.set(id, true);
    }
    bitmap
}

fn bench_bitmap(suite: &mut Suite) {
    suite.group("bitmap", |group| {
        quick(group);
        let bitmap = sample_bitmap();

        group.bench("encode_bitmap_binary_iso2", move |b| {
            b.iter(|| {
                let mut buf = [0u8; 16];
                let mut out = &mut buf[..];
                let mut scratch = [0u8; 16];
                let _ = encode_bitmap_binary_iso2(&mut out, &mut scratch, black_box(&bitmap));
                black_box(buf)
            })
        });

        group.bench("decode_bitmap_binary_iso2", |b| {
            let input: [u8; 16] = [
                0xA0, 0x20, 0x00, 0x00, 0x20, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x00, 0x00,
            ];
            b.iter(|| {
                let mut input = black_box(&input[..]);
                let mut scratch = [0u8; 16];
                black_box(decode_bitmap_binary_iso2(&mut input, &mut scratch))
            })
        });

        let bitmap = sample_bitmap();
        group.bench("encode_bitmap_ascii_hex_iso2", move |b| {
            b.iter(|| {
                let mut buf = [0u8; 32];
                let mut out = &mut buf[..];
                let mut scratch = [0u8; 32];
                let _ = encode_bitmap_ascii_hex_iso2(&mut out, &mut scratch, black_box(&bitmap));
                black_box(buf)
            })
        });

        group.bench("decode_bitmap_ascii_hex_iso2", |b| {
            let input = b"A0200000200100000000000080000000";
            b.iter(|| {
                let mut input = black_box(&input[..]);
                let mut scratch = [0u8; 32];
                black_box(decode_bitmap_ascii_hex_iso2(&mut input, &mut scratch))
            })
        });
    });
}

zenbench::main!(bench_bitmap);
