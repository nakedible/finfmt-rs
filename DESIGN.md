# finfmt-rs Design

`finfmt-rs` encodes and decodes financial wire formats such as ISO 8583 and
APACS. These formats combine byte-level operations such as padding, BCD and hex
nibble packing, EBCDIC translation, binary integer encoding, bitmaps, repeated
areas, variants, and BER-TLV sets.

The library is concerned with the binary wire format. Serde is used for normal
Rust/JSON ergonomics and for a few deliberately isolated adapter paths, but the
wire-format model is not serde-driven in general.

## Goals

- Encode and decode without heap allocation in the wire-format machinery.
- Keep all byte and bit manipulation in small primitive functions.
- Compose field formats through type-level wrappers so the optimizer sees a
  fully specialized encode/decode path at each field use site.
- Return explicit errors instead of panicking on malformed data, insufficient
  buffers, or invalid format composition.
- Make encoded data inspectable and testable through JSON-friendly Rust structs,
  without making JSON shape drive the wire-format implementation.

## Layers

`src/primitive/` contains leaf algorithms. Anything that performs actual
wire-format byte processing belongs here: copying, filling, delimiter splitting,
nibble packing, EBCDIC translation, decimal formatting/parsing, integer
encoding, BER tag/length handling, validation, and the internal bitmap data
structure.

`src/field/` contains scalar format combinators. These implement `ScalarFmt` by
chaining primitive operations with length, padding, validation, charset, nibble,
and numeric adapters. This layer should not invent new byte algorithms; it
should choose and compose primitives.

`src/composite/` contains the field dispatch traits and composite formats:
fields, lists, delimited records, bitmaps, BER-TLV sets, variants, ordered
unions, and absent/filler wrappers. Composite code may route data
between scalar formats and primitives, but byte-level conversion still belongs
in `primitive`.

Records are described by `#[wire(...)]` attributes on ordinary structs inside
`wire_type!`, which makes each one its own format. The macro reads only those
attributes and emits new items, so a derive could replace it without changing
them. Every record states its layout. The older `*_format!` macros stay until
`wire_type!` covers their layouts; both implement the same traits and nest.

`src/bitmap.rs` frames presence bitmaps: word layouts and flags, with each
word's representation a `Step`. It is the public home of `Bitmap`, whose bit
storage is a crate-internal primitive.

`src/asm/` and `benches/` are verification aids. Primitives are normally
`#[inline(always)]`, so `asm` wrappers provide `#[inline(never)]` call sites for
assembly inspection and benchmark checkpoints.

## Naming

Names use natural English word order, following the standard library
(`fmt::UpperHex`, `from_be_bytes`, `split_at_mut_checked`):

- Types put the noun that says what the item is last: `UpperHex`,
  `AsciiLength`, `BlankableEbcdicLength`, `SignPrefix`.
- Functions are verb plus object in natural order: `encode_packed_decimal_fixed`,
  `unpack_padded_nibbles`, `validate_track2_chars`.
- A qualifier that selects a behaviour variant of an existing item goes at the
  end: `pack_nibbles_checked`, `translate_bytes_inplace`, `PadLeftEven`,
  `HexEven`, `encode_packed_decimal_signed_fixed`.

Modules group related functions, and rustdoc search and editor completion
match fuzzily, so names do not repeat a family prefix to sort together. Rename
an item only when it breaks this rule, not for taste; the rule was chosen over
family-first ordering deliberately and should not be reversed name by name.

## Core Traits

`ScalarFmt` is the scalar field contract. It supports byte values, string values,
and numeric values, with `encoded_len` for exact wire-length calculation before
encoding.

`Step` is the internal scalar transformation contract used by chained scalar
formats. It converts one byte representation to another and reports the
resulting length.

`LengthSpec` describes how a length is stated: a prefix codec, a fixed length,
or nothing (`Rest`), plus arithmetic wrappers (`Offset`, `Per`). What the
length counts is the consumer's choice, never the check's units: a field
counts its rendered data at its step chain's `Count` marker, or its wire bytes
without one, and a list counts its items. Encoding computes the count by
arithmetic up to the marker (`Step::counted_len`); decoding derives the wire
length from the count by arithmetic after it (`Step::counted_wire_len`), and
the steps after the marker get their exact lengths, so padding there is split
off exactly rather than by content.

`Frame<L, Inner, Steps, MIN, MAX>` puts a group of fields or a list behind a
length and through a step chain, counted as a field counts. It has no check:
the inner fields check their own content and must produce bytes the steps
accept. Its length comes from the value, so a body outside `MIN..=MAX`, or one
`L` cannot state, is `InvalidValueLength`, where a field whose check admits a
length its framing cannot hold is a composition mistake asserted in debug
builds. `MIN` and `MAX` bound the body before the steps, for limits a
specification states beyond what the inner format and `L` imply.

`FieldEncode<T>` and `FieldDecode<'de, T>` are the format contract: a format
encodes values of type `T`, and decodes values of type `T` that may borrow
input or scratch for `'de`. Scalar and composite formats implement the same
pair, so composites take their inner formats through it and nest alike. Every
`ScalarFmt` is a format for the value types implementing `ScalarEncode` /
`ScalarDecode<'de>`: the crate provides strings, `&str` and integers
(`CompactString` behind the `compact_str` feature), and users implement them
for their own types. The value's type selects the `ScalarFmt` method (text or
typed number), so an integer field on a binary format needs no text round trip.
A value that should use its serde mapping on the wire is spelled out per field
with `SerdeScalar<F>`.

## Buffers

Encode and decode functions use advancing slices:

- Input is `&mut &[u8]`.
- Output and scratch are `&mut &mut [u8]` or `&mut [u8]`, depending on layer.
- Successful operations advance the cursor past consumed or produced bytes.
- On error, cursor and buffer contents are not guaranteed to be rolled back
  unless a caller explicitly performs a trial decode on a copied cursor.

Scratch is caller-provided workspace for intermediate representations. A decoded
value may borrow from input when no transformation is needed, or from scratch
when transformation is needed. The wire-format machinery itself should not
allocate; destination Rust types may allocate if their own representation
requires it.

Workspace never shares a region with bytes bound for output. Encoding only
appends to output; the one exception is a step chain transforming a field's
own fixed extent in place. A format that must know an encoding before writing
what precedes it, such as a length, stages it with `encode_staged`: the
encoding goes into the first half of scratch and its workspace is the second
half, and the staged bytes are copied out by the count the encoding advanced.
A length bug can then at worst ship stale staged bytes, never live workspace.
Nested staging halves scratch again, so scratch needs about `2^depth` times the
largest staged encoding.

A field writes its length before its value from the logical length its check
returns, by step and length-spec arithmetic, so scalar transformations must
have a deterministic output length for a given input length. A frame stages
its body to learn its length.

## Top-Level Buffer Strategy

Low-level encode and decode APIs use caller-provided buffers. They do not try to
allocate, grow, or choose buffer sizes themselves; insufficient output or scratch
space is reported as `BufferOverflow`.

A higher-level convenience API should usually avoid exact buffer sizing. The
normal path should pick output and scratch buffers that fit almost all messages
for the specific protocol. For messages under 1 KiB, 2 KiB of output and 8 KiB
of scratch fit three levels of staging, such as a message length around a
length-prefixed field around a token list. Flat formats fit in 4 KiB of
scratch. The right defaults are still protocol-specific.

Buffers belong to a worker thread and are reused, not allocated per message or
per connection. Clear both after each message with a plain fill, which costs
tens of nanoseconds for 8 KiB; `zeroize`'s byte-wise volatile writes cost
about 2 µs there, so keep them for when a buffer is dropped. The output needs
clearing too: a shorter message leaves the end of the previous one behind it.

If the normal path returns `BufferOverflow`, the convenience layer should retry
the whole conversion from the original input or value using a larger maximum
reasonable output buffer and scratch buffer. Examples such as 64 KiB + 64 KiB
fit many protocols with two-byte message lengths, but 128 KiB, 256 KiB, or other
limits may be the right choice for other protocols or for formats with larger
intermediate expansion.

The retry must start from scratch. Failed encode/decode operations are allowed to
partially advance cursors and mutate output or scratch buffers. Since the large
retry should be rare, it may allocate separate buffers for that attempt. If the
large retry still returns `BufferOverflow`, propagate that error like any other
conversion failure.

## Primitive Contract

Primitives are optimized for use from already validated or internally produced
data. A primitive may assume that some inputs satisfy a caller-side invariant
when that invariant has already been checked earlier in the format chain.

For invalid inputs that violate those assumptions, a primitive may return
`Error`, or it may produce invalid output. It must still not:

- panic in release builds,
- read or write outside the provided slices,
- mutate unrelated memory or unrelated cursors,
- rely on undefined behavior.

Bitmap representability is one such caller-side invariant: populated fields
must fit the layout and decoded word width, and semantic presence bits must not
be word flags. Violating these encoding preconditions
may discard fields in release builds; malformed wire input is still checked on
decode.

Debug builds should assert assumed invariants where practical. These assertions
are there to catch incorrect format composition and incorrect primitive usage
during development; they are not the release error-handling mechanism.

Public primitives should be especially careful with bounds derived from public
parameters, public traits, and public data structures. If an invalid public
parameter would otherwise cause indexing or slicing to panic, return an error or
use a harmless substitute value as appropriate for the function shape.

Release builds should not keep validation branches solely to diagnose violated
preconditions. If a check exists only to catch incorrect composition or misuse of
a prevalidated primitive, prefer `debug_assert!`. Optimized primitive code should
be allowed to assume those preconditions when doing so removes branches or other
overhead from the hot path.

### Input types

Validation comes before processing, so a validator is where a caller's value
enters the primitive layer. A validator's input type follows from what its input
can be:

- always text (a semantic value): `&str`, without UTF-8 checks;
- always encoded bytes (wire bytes, packed BCD, EBCDIC, bytes produced partway
  through a transform): `&[u8]`, so that passing a string is a compile error;
- either, with the same implementation: `impl AsRef<[u8]>`, so callers can pass
  `&str`, `String`, `CompactString` or bytes without converting;
- either, where text can skip work because it is already valid UTF-8: two
  functions, one taking `&str` and one taking `&[u8]`.

Processing primitives take `&[u8]` input, `&mut &[u8]` input cursors and
`&mut &mut [u8]` output cursors; their input usually comes from an earlier step.

## Validation

Validation happens at trust boundaries and at stages that consume untrusted wire
data. Encoding validates semantic input before transforming it. Decoding
validates wire data before a transformation depends on that representation being
well-formed.

Intermediate stages do not need redundant validation when their input was just
produced by a previous successful stage. For example, if one step decodes hex to
known-good bytes and the next step packs those bytes as BCD, the second step does
not need to repeat the original external validation just to protect itself from
our own output.

Validation functions are still primitives. They are the explicit tools for
checking byte classes, character-set representability, ranges, decimal shapes,
and fixed/even lengths where a format boundary requires that check.

Character codecs come in two deliberate kinds. A permissive byte map, such as
CP037, converts ASCII exactly and replaces anything else with SUB in both
directions, much like a lossy UTF-8 conversion; it never fails, and a field that
must be strict pairs it with a check. A strict codec, such as CP1142, converts
between UTF-8 text and the full code page and rejects unrepresentable text or
invalid UTF-8. Both keep ASCII on a fast path, since most traffic is ASCII.

### Text and number values

A format treats its value either as text or as a number.

- Text formats (a check plus steps, including BCD nibble packing) return every
  character on decode, so identifiers such as `000123` keep their leading
  zeros. Their width is counted in characters.
- Number formats (packed decimal, zoned decimal, implied decimal, and every
  typed-integer path) decode to the canonical number: no leading zeros, and
  zero is positive. When encoding they accept any spelling of a number that
  fits, so only significant digits count toward their width.

## Errors

`Error` is the scalar and primitive error type:

- `UnexpectedEof`: input ended before enough wire bytes were available.
- `BufferOverflow`: output or scratch space was too small.
- `InvalidValueLength`: when encoding, the supplied value is too long or too
  short for the field, so adding or removing characters or bytes would fix it.
  It points at the configuration or value to correct, or at a format that
  should truncate. It comes from the value's own length check. A length
  prefix or padded area too small for what the check accepts is a miswritten
  field, caught by debug assertions. Decoding never returns it: a decoded
  value of the wrong length means the incoming message was encoded wrong,
  which is `Invalid`. A typed number that does not fit is `Invalid` too, since
  it has no characters to remove.
- `Invalid`: input data was malformed or rejected by the format.
- `Internal`: format composition or library invariant was inconsistent.

`CompositeError` wraps `Error` with a short field path for composite formats.
Composite decoders should reject unknown or duplicate structural data unless a
format explicitly provides an extras/list path for preserving it.

Partial cursor advancement or partial output mutation on `Err` is allowed in
normal encode/decode paths because the whole message is rejected. Code that
needs speculative parsing must snapshot its input cursor and only commit on
success.

## Serde Boundary

Serde is an optional feature, on by default, and its support is isolated in
files with `_serde` in the name. It serves Rust/JSON ergonomics, the explicit
`SerdeScalar<F>` field adapter, and structural BER-TLV list/map decoding
(`BerTlvList`). Nothing else requires it: field dispatch uses the value traits,
and the crate builds without serde.

The general structural wire-format path is not serde-based. Serde concepts such
as flattening and optional field handling do not map cleanly to bitmap-driven,
delimiter-driven, fixed-layout, or variant wire formats. Those are represented
by explicit field format implementations and macros. Serde attributes on a
value type describe its JSON form; they reach the wire only through an explicit
`SerdeScalar<F>`, and never for enums: the adapter rejects them, because an
enum's wire mapping is its own `ScalarEncode`/`ScalarDecode` implementation.

## Composite Semantics

Bitmap formats encode fields in field-number order. Decode uses the bitmap to
decide which fields are present. If a field is absent, the corresponding Rust
field must be represented by the composite format shape, usually as an optional
field or through a defined absent, optional, or union representation.

Pattern-based absence must be explicit. A value whose bytes are always on the
wire can still be absent: a blank-filled fixed area, a zero date, COBOL
low-values. `OptionAs<Inner, Absent>` maps `Option<T>` onto such a field: the
`Absent` encoding (an `AbsentFmt`, such as `AbsentBytes<Fill<b' ', 8>>`) is
wire bytes, matched before `Inner` sees the input, because `Inner` often cannot
represent them; anything else decodes through `Inner`, whose errors are
returned rather than read as absence. Usually the present-side format cannot
encode the absent bytes. When it can, as with a zero amount, `Some(value)` and
`None` encode identically and the value reads back as absent: that is the
format's definition, not a generic rule.

BER-TLV named-field formats reject duplicate known tags. Unknown tags are
rejected by default. Formats with an extras path preserve unknown tags as
uppercase hex strings, and list-style BER-TLV formats preserve order and
duplicates.

Repeated and delimited formats decode each item through its own format and
reject trailing bytes inside an item unless that item format explicitly
consumes them.

Union and literal matching paths are allowed to decode speculatively. They must
snapshot input cursors before trial decoding and only advance the input for the
successful path. Union decode tries arms in order. `Invalid` and
`UnexpectedEof` mean "try the next arm"; `BufferOverflow` and `Internal` are
fatal. Decoding never returns `InvalidValueLength`. Scratch is arena space for borrowed
decode, so speculative failure may consume scratch before a later union arm
succeeds. Owned union decode can retry each arm with the original scratch slice.

## Optimization Policy

The intended hot path is a fully monomorphized field encoder or decoder. Format
parameters come from generic field types, so branchy primitive signatures are
acceptable when the optimizer can fold those parameters at the call site.

Prefer fewer primitive entrypoints when the optimized code remains equivalent.
Do not add separate fixed/variable or fallible/infallible variants solely for
source-level neatness. Split code when calling convention, algorithm, or
optimization evidence justifies it.

Primitive functions are normally `#[inline(always)]`. Cold error handling should
call `cold_path()` so the success path stays fall-through.

## Verification

The standard guard is `make verify`: formatting, clippy, tests, and a release
build with all features.

The `no-panic` feature adds release link-time panic checks to primitives where
the optimizer can prove panic-freedom. It is not intended for debug builds,
because debug builds keep bounds and overflow checks that the optimizer would
normally eliminate.

The `asm-inspect` feature exposes stable wrapper symbols under `finfmt::asm::*`
so `cargo asm` can inspect the specialized machine code emitted for typical
primitive use sites.

Benchmarks call the same `asm` wrappers and act as regression checks for the
primitive layer.

## Deferred Design Questions

Some earlier design goals are not part of the current contract and should be
decided separately before being documented as guarantees:

- preferred owned string type for generated or example structs,
- exact charset scope beyond the currently implemented encodings,
- whether little-endian binary integers belong in this crate,
- how much allocation policy belongs to this library versus user-owned Rust
  types,
- whether serde should become optional behind a Cargo feature.

## Out Of Scope

- Message transport and framing around complete messages.
- Protocol headers such as TPDU unless modeled explicitly by a user format.
- Business semantics of fields and bitmap bits.
- Default semantic values for absent fields unless a composite format explicitly
  defines an absent, union, or optional representation.
