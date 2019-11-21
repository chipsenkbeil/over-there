# Over There

Tool to enable software management and execution remotely from "over there."

## Making a release

See the following link about file size:
https://stackoverflow.com/a/54842093

```
cargo build --release
strip target/release/over-there
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
