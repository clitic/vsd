<h1 align="center">vsd-mp4</h1>

<p align="center">
  <a href="https://crates.io/crates/vsd-mp4">
    <img src="https://img.shields.io/crates/d/vsd-mp4?style=flat-square">
  </a>
  <a href="https://crates.io/crates/vsd-mp4">
    <img src="https://img.shields.io/crates/v/vsd-mp4?style=flat-square">
  </a>
  <a href="https://docs.rs/vsd-mp4">
    <img src="https://img.shields.io/docsrs/vsd-mp4?logo=docsdotrs&style=flat-square">
  </a>
  <a href="https://github.com/clitic/vsd/blob/main/vsd-mp4/README.md#license">
    <img src="https://img.shields.io/crates/l/vsd-mp4?style=flat-square">
  </a>
</p>

This crate contains a mp4 parser ported from [shaka-player](https://github.com/shaka-project/shaka-player) project. Also, some optional features are added for parsing subtitles, `PSSH` and `SIDX` boxes.

## Getting Started

Add this to your Cargo.toml file.

```toml
[dependencies]
vsd-mp4 = "0.1.4"
```

Or add from command line.

```bash
$ cargo add vsd-mp4
```

See [docs](https://docs.rs/vsd-mp4) and [examples](https://github.com/clitic/vsd/tree/main/vsd-mp4/examples) to 
know how to use it.

## License

Dual Licensed

- [Apache License, Version 2.0](https://www.apache.org/licenses/LICENSE-2.0) ([LICENSE-APACHE](LICENSE-APACHE))
- [MIT license](https://opensource.org/licenses/MIT) ([LICENSE-MIT](LICENSE-MIT))
