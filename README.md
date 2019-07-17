# Cthulhu

Generate FFI-compatible `extern fn` from harmless `fn`s using the power of proc
macros. If you are wondering which ABI this is, remember: You can't spell
Cthulhu without C!

## Usage

Invoke the hive mind of chaos by adding the `#[cthulhu]` to your function.

### "Supported" automatic conversion

- [x] `bool` to `c_char`
- [x] Boring number conversions as defined in [`std::os::raw`](https://doc.rust-lang.org/1.36.0/std/os/raw/index.html)
- [x] `Arc<str>` to `(*const char, usize)`
- [x] `&'a CStr` to `*const char`
- [x] `CString` to `*mut char`

Interesting reads:

- [Marshaling Data with Platform Invoke](https://docs.microsoft.com/en-us/dotnet/framework/interop/marshaling-data-with-platform-invoke) (.NET)

## Why?

[@bbqsrc](https://github.com/bbqsrc) made me do it.
You might even say it's [cursed](https://github.com/bbqsrc/cursed).

## Is it any good?

Oh my sweet summer child…

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
