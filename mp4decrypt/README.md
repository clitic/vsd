<h1 align="center">mp4decrypt</h1>

<p align="center">
  <a href="https://crates.io/crates/mp4decrypt">
    <img src="https://img.shields.io/crates/d/mp4decrypt?style=flat-square">
  </a>
  <a href="https://crates.io/crates/mp4decrypt">
    <img src="https://img.shields.io/crates/v/mp4decrypt?style=flat-square">
  </a>
  <a href="https://docs.rs/mp4decrypt">
    <img src="https://img.shields.io/docsrs/mp4decrypt?logo=docsdotrs&style=flat-square">
  </a>
  <a href="https://github.com/clitic/vsd/blob/main/mp4decrypt/README.md#license">
    <img src="https://img.shields.io/crates/l/mp4decrypt?style=flat-square">
  </a>
</p>

This crate provides a safe high-level API to decrypt CENC/CENS/CBC1/CBCS encrypted MP4 data using [Bento4](https://github.com/axiomatic-systems/Bento4).

## Getting Started

Add this to your Cargo.toml file.

```toml
[dependencies]
mp4decrypt = "0.6.0"
```

Or add from command line.

```bash
$ cargo add mp4decrypt
```

See [docs](https://docs.rs/mp4decrypt) and [examples](https://github.com/clitic/vsd/tree/main/mp4decrypt/examples) to 
know how to use it.

## License

Dual Licensed

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) ([LICENSE-APACHE](LICENSE-APACHE))
- [MIT license](https://opensource.org/licenses/MIT) ([LICENSE-MIT](LICENSE-MIT))
