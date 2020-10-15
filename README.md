# cffi

Use the `#[cffi::marshal(...)]` proc macro to make unsafe C functions into much safer functions,
both for exporting interfaces and consuming functions from C libraries.

## Usage

See the documentation for the various marshallers available.

## Where is this used?

- [pahkat](https://github.com/divvun/pahkat) - a multi-platform package management framework
- [divvunspell](https://github.com/divvun/divvunspell) - a multi-platform highly efficient memory-mapping spell checking library

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
