# Over There &emsp; [![Build Status]][actions] [![Latest Version]][crates.io] [![Latest Docs]][docs.rs] [![over-there: rustc 1.39+]][Rust 1.39]

[Build Status]: https://img.shields.io/github/workflow/status/chipsenkbeil/over-there/CI/master
[actions]: https://github.com/chipsenkbeil/over-there/actions?query=branch%3Amaster
[Latest Version]: https://img.shields.io/crates/v/over-there.svg
[crates.io]: https://crates.io/crates/over-there
[Latest Docs]: https://docs.rs/over-there/badge.svg
[docs.rs]: https://docs.rs/over-there
[over-there: rustc 1.39+]: https://img.shields.io/badge/over--there-rustc_1.39+-lightgray.svg
[Rust 1.39]: https://blog.rust-lang.org/2019/11/07/Rust-1.39.0.html

**Over There is a library and tool to enable software management and execution remotely from "over there."**

## Building developer version

By default, the CLI feature is not included. This means that executing a
normal build will not include the binary:

```
cargo build
```

Instead, the *cli* feature must be specified:

```
cargo build --features 'cli'
```

## Making a release

See the following link about file size:
https://stackoverflow.com/a/54842093

```
cargo build --release --features 'cli'
strip target/release/over-there
```

## Making a release without dynamically linking libc

```
rustup target add x86_64-unknown-linux-musl
cargo build --release --target=x86_64-unknown-linux-musl --features 'cli'
```

Note that on Mac OS X you will need to install *musl-gcc*:

```
brew install FiloSottile/musl-cross/musl-cross
```

And to do a strip (on Mac), use the musl strip:

```
x86_64-linux-musl-gcc target/x86_64-unknown-linux-musl/release/over-there
```

## Notes on running

One obvious one is that you need to match server IPv4 with client IPv4 and
server IPv6 with client IPv6.

E.g. The following works fine between IPv6
```
# On your machine
over-there client '[1111:2222:3333:4444:5555:6:78:9]:60000' <some command>

# On 1111:2222:3333:4444:5555:6:78:9
over-there server '[::]:60000'
```

E.g. The following works fine between IPv4
```
# On your machine
over-there client '123.456.7.890:60000' <some command>

# On 123.456.7.890
over-there server '0.0.0.0:60000'
```

## overthered

`otd` as alias?

Daemon wrapper around `overthere` that runs service to listen for requests
and execute them.

## overtherec

`otc` as alias?

Client wrapper around `overthere` that can send commands to a remote daemon
to execute and can relay results in a variety of means like stdout or
files.

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
