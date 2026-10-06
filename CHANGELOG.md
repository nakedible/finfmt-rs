# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/nakedible/finfmt-rs/compare/v0.1.1...v0.2.0) - 2026-10-06

### Added

- [**breaking**] MulLen, and rename Per and Offset to DivLen and AddLen
- [**breaking**] generic single-byte code pages
- add FixedSignedZonedAscii and SignSuffix
- add IsoBitmap, BitsBitmap and FixedBitmap descriptors
- [**breaking**] make optional constants bool flags, and add #[wire(skip)]
- frame whole TLV entries with tlv(entry = Frame<L, TlvEntry>)
- add a serde helper for unknown TLV tag keys
- match TLV tags by their encoded bytes
- add generic tag-length-value records to wire_type!
- accept alias keys for selected enum variants
- select by a function of earlier fields, and on Option fields
- recognize std and core Option paths in wire_type! fields
- name a selected enum's variant with wire_name()
- decode a frame's body with a context
- add unit and selected enums to wire_type!
- add absent forms to wire_type!
- add fixed fields to wire_type!
- add FixedValue and FixedBytes formats for () fields
- add the BER-TLV layout to wire_type!
- add the bitmap layout to wire_type!
- add the delimited layout to wire_type!
- add wire_type! for concat records
- add Per for lengths that count groups of units
- key BerTlvList entries by their plain uppercase tag
- add Offset for lengths that count more than the payload
- flag bitmap words with global field numbers
- add Check::validate_str for text known to be UTF-8
- expose validate_bytes for custom checks
- record list indices in composite error paths
- make BER zero padding opt-in
- support usize in direct scalar adapters
- add explicit byte truncation field adapter
- add optional CP037 ASCII wire validation

### Changed

- [**breaking**] depend on compact_str 0.10 and raise dependency minimums
- [**breaking**] absence belongs to the format; `required` marks always-on-wire options
- [**breaking**] let delimited Option fields mean a left-out tail
- [**breaking**] replace BerTlvList with a generic TlvList
- [**breaking**] express BER-TLV records as tlv(tag = BerTag)
- let TLV fields carry their own lengths
- port the NIBSS example to wire_type!
- parse wire_type! field arguments in one place
- frame groups by a length spec and steps
- stage encodes in scratch halves, never in output
- match absent values as wire bytes with OptionAs
- dispatch fields by the value's type, without serde
- pad in place after a transforming step
- count lengths at an explicit point in the step chain
- leave repeated unknown BER-TLV tags to the extras collection
- exclude known tags from BER-TLV extras by their hex
- stage BER-TLV values instead of moving them into place
- match BER-TLV tags by their hex bytes
- frame lists with length specs like fields
- slice CompositeError paths without a panic path
- frame bitmaps in src/bitmap.rs behind one public path
- check bitmap layouts at compile time and in debug builds only
- represent bitmap words with a Step and put the width in the layout
- drop the runtime field range check in Bitmap get and set
- stop retrying union arms on value length errors
- check only content where decoded lengths are already exact
- debug-assert digit fit in the packed and zoned digit encoders
- check decimal prefix widths with one primitive
- encode PaddedField values straight into the output
- reject impossible numeric format parameters at build time
- simplify parse_ber_tag_hex error mapping
- add encode_ber_tlv_head for known-length values
- name decimal primitives in natural order
- decode zoned digits arithmetically
- debug-assert numeric format parameters
- name padding primitives by the fill side
- match validator input types to their input side
- put case first in hex check names
- rename pack_expanded_nibbles to pack_nibbles_checked
- pass nibble alphabets as type parameters
- encode CP1142 steps with the strict encoder
- rename and trim byte primitives
- share reserve and take byte cursor primitives
- split CompositeFmt cursor methods from whole-message functions
- treat encode scratch as a per-call workspace
- make CompositeError truncation flag private
- move compact_str to dev-dependencies
- move in-place BER framing into a primitive
- encode delimited fields directly into output
- consolidate BER textual adapters
- use fixed-size chunks for absent slots
- name composite errors and decode helper accurately
- group decimal analysis and clarify codec names
- encode fixed decimal digits directly
- name decoded bitmap word width explicitly
- assert bitmap representability in debug builds
- reuse validated CP1142 text in field encoding
- decode fields through one advancing scratch cursor
- reuse validated digits in implied decimal fields
- compose in-place encoding in step chains
- clarify BER primitive names and limits
- bound packed and zoned decimal transforms
- share packed sign handling and simplify prefix decoding
- share decimal digit parsing and fixed codecs
- strip padding from framed byte slices
- separate byte truncation from padding
- clarify validator APIs and share character validation
- derive CP1142 tables and use direct character lookup
- express nibble pairs with fixed-size chunks
- define nibble alphabets with derived reverse mappings
- separate delimiter splitting from field policy
- clarify padded byte and repeated block names
- reuse primitive byte copying in field encoders

### Fixed

- reject the end-of-contents identifier when decoding BerTag
- see a character-counting step behind Count or Identity
- reject cfg on items, fields and variants in wire_type!
- key generated field constants by position on the record
- match an absent pattern against all of a record taking the rest
- build and test without serde
- reject character-counting steps in a frame
- assert every list length its framing cannot state
- reject a length Fixed or Per cannot state
- reject enums in the serde scalar adapter
- report over-long BER values and odd extras hex as value length errors
- reuse decode scratch for BER-TLV text of each entry
- report a wrong number of list items as a value length error
- write absent areas and fixed slots sequentially
- check list and delimited separators in debug builds only
- cap list pre-allocation by the remaining input
- treat framing too small for its check as a composition error
- assert in debug builds that Fixed values fill their width
- size counting prefixes from the padded width
- treat framing counts as widths in value units
- reject double signs and keep zero positive in sign wrappers
- make PadLeftEven and PadRightEven decoding bounded and exact
- count significant digits in packed and zoned decimals
- report value length errors only when encoding
- encode negative zero as positive
- truncate text by characters
- report wrong-length fixed integer values as value length errors
- trial-decode tagged literals before selecting variants
- allow separators in final delimited record fields
- reject duplicate declared BER tags at compile time
- validate bitmap record declarations and unknown fields
- support borrowed DirectScalar record fields
- complete BER Serde pair handling
- format BER strings in caller scratch
- reject declared BER tags in extras
- decode wire-length prefixes in trailing frames
- honor absence matchers in fixed-area lists
- reject excessive list counts before allocation
- preserve list item boundaries and empty values
- reuse temporary absence comparison workspace
- preserve borrowing in serde enum identifiers
- format serde scalar values into caller scratch
- report bitmap word decoder contract violations
- support required and optional bitmap words
- assert field encoders emit their predicted size
- validate prefix length predictions and declared extents
- assert fixed field framing compatibility in debug builds
- clarify step bounds and decode validation
- remove numeric scratch limits and check decimal widths
- align numeric field dispatch and length checks
- prove parsed BER tag slicing cannot panic
- reject end-of-contents as BER data entries
- validate complete textual BER tags
- bound implied decimal layouts and reserve exact scratch
- bound integer codecs and honor nibble zero aliases
- validate hex content before even-length constraints
- reject short output buffers in byte translation
- correct CP037 ASCII control and punctuation mappings
- make repeated block loops provably panic-free
- preserve leaf fields in truncated error paths

### Other

- port record tests to wire_type!
- say Truncate cuts in the value's own units
- state the entry order ber_tlv_format! encodes
- benchmark whole NIBSS messages
- cover bitmap representations, arbitrary input and more layouts
- document length specs and PaddedField
- explain that in-place step encoding is for speed
- cite X.690 and EMV for BER decoding rules
- cite IBM for accepted decimal sign codes
- document every check
- describe permissive and strict EBCDIC codec roles
- format private macro helpers
- add make check and run it in CI
- enforce panic freedom for prevalidated ASCII helpers

### Removed

- [**breaking**] remove the old composite format macros
- remove WireFixed
- remove Step::decoded_max_len
- remove unused text and range primitives
- remove unused private BER macro helpers
- remove unused validate_all_bytes helper

## [0.1.1](https://github.com/nakedible/finfmt-rs/compare/v0.1.0...v0.1.1) - 2026-07-09

### Changed

- update dependency versions

### Other

- Fix clippy byte slice warnings

## [0.1.0] - 2026-07-09

### Added

- Initial public release.
