# Cthulhu

Generate FFI-compatible `extern fn` from harmless `fn`s using the power of proc
macros. If you are wondering which ABI this is, remember: You can't spell
Cthulhu without C!

## Usage

It's a mystery.

## Roadmap

### v0.1 series

Let's get this show on the road.

- [x] Introduce marshalers and their `ToForeign` and `FromForeign` traits
- [x] Add procedural macros for invoking marshalers
- [x] Generate safe runtime checking of unsafe boundaries
- [x] Generate "exception handling" callbacks 
- [x] Handle all basic scalar primitives (including the bool special case)
- [x] Introduce `ReturnType` trait for reflecting return values for complex types
- [x] Encapsulate all information needed to generate functions into a single `Function` type
- [ ] Provide a micro-framework for demonstrating Cthulhu error handling

### v0.2 series

Let's get more show out of this road.

- [ ] Implement a good resource cleanup story
- [ ] Implement a good `Vec<T>` story
- [ ] Implement a good ref/owned/ohno story
- [ ] Get strings in all their forms working safely and ergonomically
- [ ] Allow generating extern functions by `invoke`ing on `impl` and `mod` levels
  - [ ] Auto-prefixing of functions with a "C namespace" of the user's choice
- [ ] Experiment with other syntaxes for declaring marshalers on longer type signatures
- [ ] Improve error handling and reporting (some spans are still garbage or wrong)
- [ ] Supply default marshalers for:
  - [ ] Path types per operating system
  - [ ] UTF-16 owned/borrowed strings
  - [ ] UTF-8 owned/borrowed strings
  - [ ] `Arc<T>`
- [ ] Make `invoke` syntax consistent with `marshal`
- [ ] Add debug logging to inform the user when a value has been consumed and should not be reused

### v1.0 series

Let's get this stable.

- [ ] Ability to generate APIs for Kotlin, Swift and C#

## Background and philosophy

We enforce the usage of `stdint.h` types on the C side to simplify the implementation on the Rust side.

Interesting reads:

- [Marshaling Data with Platform Invoke](https://docs.microsoft.com/en-us/dotnet/framework/interop/marshaling-data-with-platform-invoke) (.NET)

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
