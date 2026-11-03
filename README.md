# cpg-rs
Code pages like Windows-1250, Mazovia and many others, in Rust with no unsafe code and no
dependencies. It reads bytes from untrusted sources, such as web pages, and turns them into UTF-8.
It is a port of [cpg](https://github.com/eudahub/cpg), with the same names for code pages.

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
Detecting the code page of a text, and converting from UTF-8 back to a code page, are not here yet.
