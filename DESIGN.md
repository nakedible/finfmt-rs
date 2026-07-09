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

`src/composite/` contains composite formats. `CompositeFmt<T>` maps Rust values
to and from fields, lists, delimited records, bitmaps, BER-TLV sets, variants,
ordered unions, and absent/filler wrappers. Composite code may route data
between scalar formats and primitives, but byte-level conversion still belongs
in `primitive`.

`src/asm/` and `benches/` are verification aids. Primitives are normally
`#[inline(always)]`, so `asm` wrappers provide `#[inline(never)]` call sites for
assembly inspection and benchmark checkpoints.

## Core Traits

`ScalarFmt` is the scalar field contract. It supports byte values, string values,
and numeric values, with `encoded_len` for exact wire-length calculation before
encoding.

`Step` is the internal scalar transformation contract used by chained scalar
formats. It converts one byte representation to another and reports the
resulting length.

`LengthSpec` describes how a field length is encoded or inferred: fixed, rest,
wire-length prefixed, semantic-length prefixed, and related wrappers.

`CompositeFmt<T>` is the composite contract. It encodes and decodes complete Rust
values using caller-provided output and scratch buffers.

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

Length-prefixed formats rely on `encoded_len` to write the length before the
value. Supported scalar transformations should have deterministic output length
for a given semantic input.

## Top-Level Buffer Strategy

Low-level encode and decode APIs use caller-provided buffers. They do not try to
allocate, grow, or choose buffer sizes themselves; insufficient output or scratch
space is reported as `BufferOverflow`.

A higher-level convenience API should usually avoid exact buffer sizing. The
normal path should pick output and scratch buffers that fit almost all messages
for the specific protocol. Examples such as 2 KiB + 2 KiB or 4 KiB + 4 KiB are
often already larger than real financial messages, but the right defaults are
protocol-specific.

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
- rely on undefined behavior,
- silently lose structural data, such as truncating a bitmap outside its layout.

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

## Errors

`Error` is the scalar and primitive error type:

- `UnexpectedEof`: input ended before enough wire bytes were available.
- `BufferOverflow`: output or scratch space was too small.
- `InvalidValueLength`: semantic value length did not satisfy the format.
- `Invalid`: input data was malformed or rejected by the format.
- `Internal`: format composition or library invariant was inconsistent.

`StructError` wraps `Error` with a short field path for composite formats.
Composite decoders should reject unknown or duplicate structural data unless a
format explicitly provides an extras/list path for preserving it.

Partial cursor advancement or partial output mutation on `Err` is allowed in
normal encode/decode paths because the whole message is rejected. Code that
needs speculative parsing must snapshot its input cursor and only commit on
success.

## Serde Boundary

Serde support is intentionally isolated in files with `_serde` in the name where
possible. It exists for Rust/JSON ergonomics and for special generic adapter
cases such as BER-TLV list/map decoding.

The general structural wire-format path is not serde-based. Serde concepts such
as flattening and optional field handling do not map cleanly to bitmap-driven,
delimiter-driven, fixed-layout, or variant wire formats. Those are represented
by explicit `CompositeFmt` implementations and macros.

## Composite Semantics

Bitmap formats encode fields in field-number order. Decode uses the bitmap to
decide which fields are present. If a field is absent, the corresponding Rust
field must be represented by the composite format shape, usually as an optional
field or through a defined absent, optional, or union representation.

Pattern-based absence must be explicit. A format may define bytes that mean an
optional value is absent, for example a blank-filled fixed area, an impossible
length prefix, or a structured pattern matching a legacy initialized area. The
present-side format should normally be unable to encode those bytes. If
`Some(value)` and `None` encode identically, the mapping is lossy and should be
a deliberate domain decision, not a generic default-derived rule.

BER-TLV named-field formats reject duplicate known tags. Unknown tags are
rejected by default. Formats with an extras path preserve unknown tags as
uppercase hex strings, and list-style BER-TLV formats preserve order and
duplicates.

Repeated and delimited formats decode each item through its own `CompositeFmt` or
`ScalarFmt` and reject trailing bytes inside an item unless that item format
explicitly consumes them.

Union and literal matching paths are allowed to decode speculatively. They must
snapshot input cursors before trial decoding and only advance the input for the
successful path. Union decode tries arms in order. `Invalid`,
`InvalidValueLength`, and `UnexpectedEof` mean "try the next arm";
`BufferOverflow` and `Internal` are fatal. Scratch is arena space for borrowed
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
