
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
- [ ] Clean up the tests and make them pass
- [ ] Support types whose representation in C would be multiple parameters or a struct

### v1.0 series

Let's get this stable.

- [ ] Ability to generate APIs for Kotlin, Swift and C#
- [ ] Fuzzed so hard that the code is no longer fazed by aggressive abuse and invalid files
