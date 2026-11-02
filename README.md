[![Actions Status](https://github.com/cffi-rs/cffi/workflows/CI/badge.svg)](https://github.com/cffi-rs/cffi/actions)
[![Documentation](https://docs.rs/cffi/badge.svg)](https://docs.rs/cffi)

# cffi

Use the `#[cffi::marshal(...)]` proc macro to make unsafe C functions into much safer functions,
both for exporting interfaces and consuming functions from C libraries.

## Usage

See the documentation for the various marshallers available.

### Freeing

What a marshaler hands to foreign code belongs to the caller until it is given back:

- a string: `cffi_string_free`
- a `Vec<u8>`: `cffi_vec_free`
- a `Vec<T>` of any other `T`: a free function of your own that calls `VecMarshaler::<T>::free`
- an `Arc<T>`: a free function of your own that calls `ArcMarshaler::<T>::free`
- a path from `PathBufMarshaler`, UTF-16 on Windows and bytes elsewhere: a free function of your
  own that calls `PathBufMarshaler::free`

Each of these does nothing when given NULL, and catches a panic in a destructor.

### Callbacks and their context

A function `#[cffi::marshal]` makes that can fail takes, after its own arguments, an exception
callback, `void (*)(const uint8_t *msg, size_t msg_len, void *context)`, and last of all a
`void *context`, which it hands back to the callback untouched. A C function pointer captures
nothing, so the context is how the callback reaches somewhere of the caller's to put the message.
A return callback, where a function has one, comes before the context and is handed the same one.

### Panics

A function `#[cffi::marshal]` makes catches a panic in its work rather than let it unwind into
foreign code, reports it through the exception callback as a `cffi::Panic`, and returns the zero
value of its return type. This needs `panic = "unwind"`, the default.

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
