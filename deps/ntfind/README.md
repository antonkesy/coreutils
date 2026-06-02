# DOS find, ported to NT, ported to Rust

A Rust port of find.exe that has shipped since Windows NT.

It was ported from C++ to Rust, as the original depends on an internal library named "ulib".
Open sourcing ulib was impractical, because it is a very large library of substandard quality.
Unfortunately, due to this it's difficult to guarantee that this port is 100% faithful.

## Build

Recommended compilation command:

```sh
# If using stable Rust, set RUSTC_BOOTSTRAP=1
cargo build --release --config ../../.cargo/release.toml
```

The resulting binary will be within a few KB of the original.
