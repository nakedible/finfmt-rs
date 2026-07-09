pub use crate::primitive::bitmap::{Bitmap, BitmapLayout, BitmapWord, decode_bitmap, encode_bitmap};
use crate::{Binary, Field, Fixed, Step};

impl<S: Step, const N: usize> BitmapWord for Field<Binary<N, N>, Fixed<N>, S> {
    const BYTES: usize = N;
}

#[cfg(test)]
mod tests {
    use super::*;

    type BitmapBinaryHalfWord = crate::Field<crate::Binary<4, 4>, crate::Fixed<4>>;
    type BitmapBinaryWord = crate::Field<crate::Binary<8, 8>, crate::Fixed<8>>;
    type BitmapAsciiHexWord = crate::Field<crate::Binary<8, 8>, crate::Fixed<8>, crate::UnpackNibbles<crate::primitive::nibble::HexUpper>>;

    #[test]
    fn test_bitmap_bits_and_words() {
        let mut bitmap = Bitmap::new();
        bitmap.set(1, true);
        bitmap.set(64, true);
        bitmap.set(65, true);
        bitmap.set(192, true);
        assert!(bitmap.get(1) && bitmap.get(64) && bitmap.get(65) && bitmap.get(192));
        assert_eq!(bitmap.highest_word(), 2);
    }

    #[test]
    fn test_bitmap_ascii_hex_roundtrip() {
        let mut bitmap = Bitmap::new();
        bitmap.set(3, true);
        bitmap.set(63, true);
        bitmap.set(97, true);
        let layout = BitmapLayout::new(3, [Some(64), Some(64), None]);
        let mut output = [0u8; 64];
        let mut scratch = [0u8; 8];
        let used = {
            let total = output.len();
            let mut out_ptr = output.as_mut_slice();
            encode_bitmap::<BitmapAsciiHexWord>(&mut out_ptr, &mut scratch, &bitmap, layout).unwrap();
            total - out_ptr.len()
        };
        let mut input = &output[..used];
        let decoded = decode_bitmap::<BitmapAsciiHexWord>(&mut input, &mut scratch, layout).unwrap();
        assert_eq!(decoded, bitmap);
        assert!(input.is_empty());
    }

    #[test]
    fn test_bitmap_ascii_hex_rejects_invalid_digits() {
        let layout = BitmapLayout::new(1, [None, None, None]);
        let mut input = b"000000000000000G".as_slice();
        let mut scratch = [0u8; 8];
        assert_eq!(
            decode_bitmap::<BitmapAsciiHexWord>(&mut input, &mut scratch, layout),
            Err(crate::Error::Invalid)
        );
    }

    #[test]
    fn test_bitmap_binary_half_word_roundtrip() {
        let mut bitmap = Bitmap::new();
        bitmap.set(2, true);
        bitmap.set(3, true);
        bitmap.set(4, true);
        let layout = BitmapLayout::new(1, [None, None, None]);
        let mut output = [0u8; 16];
        let mut scratch = [0u8; 8];
        let used = {
            let total = output.len();
            let mut out_ptr = output.as_mut_slice();
            encode_bitmap::<BitmapBinaryHalfWord>(&mut out_ptr, &mut scratch, &bitmap, layout).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..used], &[0x70, 0x00, 0x00, 0x00]);
        let mut input = &output[..used];
        let decoded = decode_bitmap::<BitmapBinaryHalfWord>(&mut input, &mut scratch, layout).unwrap();
        assert_eq!(decoded, bitmap);
        assert!(input.is_empty());
    }

    #[test]
    fn test_bitmap_fixed_two_word_roundtrip() {
        let mut bitmap = Bitmap::new();
        bitmap.set(3, true);
        bitmap.set(65, true);
        let layout = BitmapLayout::new(2, [None, None, None]);
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 8];
        let used = {
            let total = output.len();
            let mut out_ptr = output.as_mut_slice();
            encode_bitmap::<BitmapBinaryWord>(&mut out_ptr, &mut scratch, &bitmap, layout).unwrap();
            total - out_ptr.len()
        };
        let mut input = &output[..used];
        let decoded = decode_bitmap::<BitmapBinaryWord>(&mut input, &mut scratch, layout).unwrap();
        assert_eq!(decoded, bitmap);
        assert!(input.is_empty());
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "bitmap max_words out of range")]
    fn test_bitmap_invalid_layout_debug_asserts() {
        let bitmap = Bitmap::new();
        let layout = BitmapLayout::new(4, [None, None, None]);
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 8];
        let mut out = output.as_mut_slice();
        let _ = encode_bitmap::<BitmapBinaryWord>(&mut out, &mut scratch, &bitmap, layout);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic(expected = "bitmap contains words outside layout")]
    fn test_bitmap_outside_layout_debug_asserts() {
        let mut bitmap = Bitmap::new();
        bitmap.set(65, true);
        let layout = BitmapLayout::new(1, [None, None, None]);
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 8];
        let mut out = output.as_mut_slice();
        let _ = encode_bitmap::<BitmapBinaryWord>(&mut out, &mut scratch, &bitmap, layout);
    }

    #[cfg(not(debug_assertions))]
    #[test]
    fn test_bitmap_invalid_inputs_return_errors_in_release() {
        let mut output = [0u8; 32];
        let mut scratch = [0u8; 8];
        let mut out = output.as_mut_slice();
        assert_eq!(
            encode_bitmap::<BitmapBinaryWord>(&mut out, &mut scratch, &Bitmap::new(), BitmapLayout::new(4, [None, None, None])),
            Err(crate::Error::Internal)
        );

        let mut bitmap = Bitmap::new();
        bitmap.set(65, true);
        let mut out = output.as_mut_slice();
        assert_eq!(
            encode_bitmap::<BitmapBinaryWord>(&mut out, &mut scratch, &bitmap, BitmapLayout::new(1, [None, None, None])),
            Err(crate::Error::Internal)
        );
    }
}
