# cpg-rs
Code pages like Windows-1250, Mazovia and many others, in Rust with no unsafe code, and no ICU. It
reads bytes from untrusted sources, such as web pages, finds out which code page they are in, and
turns them into UTF-8. It is a port of [cpg](https://github.com/eudahub/cpg), with the same names
for code pages. Its only dependencies are also in Rust: `miniz_oxide` to unpack the language models,
and `unicode-general-category` for what kinds of characters there are.

```rust
let mazovia = cpg::by_name("mazovia").unwrap();
assert_eq!(mazovia.to_utf8(b"\x86").unwrap(), "ą");

// When a few bad bytes should not cost the whole text:
let text = cpg::by_name("cp1253").unwrap().to_utf8_lossy(b"a\xAAb");
assert_eq!(text.utf8, "a\u{FFFD}b");
assert!(text.replaced);
```

It has UTF-8, UTF-16 and UTF-32 in both byte orders, and all single-byte code pages of cpg. As in
cpg, the tables only cover bytes from 128 up, and bytes below that are ASCII in every code page.
Converting from UTF-8 back to a code page is not here yet.

## Detecting the code page

```rust
use cpg::Detector;

let bytes = b"Za\xBF\xF3\xB3\xE6 g\xEA\x9Cl\xB9 ja\x9F\xF1, a to jest polski tekst.";
// Unpacks the models, which take some memory, so keep it only for as long as it is needed.
let detector = Detector::new();
let results = detector.detect(bytes);
let alternatives = Detector::plausible(&results, 0.95);
if let [only] = alternatives.as_slice() && only.confidence >= 0.95 {
    println!("{} in {}", only.codepage, only.language);
} else {
    // Not certain: show the alternatives, or take the first one and say so.
}
```

Each candidate gets a probability of giving the right text, from models of the characters of 32
languages, which are built into the library (`data/models`, 722 KB). Decodings that give the same
text, such as cp1250 and iso-8859-2 without a byte that differs, share one probability. Valid UTF-8
without a byte order mark competes with the code pages, since for example Polish "Ół" in cp1250 is
valid UTF-8 too, and each of its multi-byte sequences counts in its favour. UTF-8 with a byte order
mark, UTF-16 and UTF-32 are recognized without the models, which then only tell the language.

Results follow those of cpg, whose documentation tells how they were calibrated. cpg asks ICU about
Unicode; here the same checks for Unicode are built in, and the other encodings ICU knows do not
compete with them.
