# Over There

Tool to enable software management and execution remotely from "over there."

## Making a release

See the following link about file size:
https://stackoverflow.com/a/54842093

```
cargo build --release --bin over-there
strip target/release/over-there
```

## Making a release without dynamically linking libc

```
rustup target add x86_64-unknown-linux-musl
cargo build --release --bin over-there --target=x86_64-unknown-linux-musl
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
