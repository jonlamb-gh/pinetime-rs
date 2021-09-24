# pinetime-rs

Rust & [RTIC](https://rtic.rs/dev/book/en/) running on the PineTime watch.

See the [PineTime Wiki](https://wiki.pine64.org/index.php/PineTime) for docs.

```bash
cargo install probe-run cargo-embed flip-link
```

## Tests

```bash
# ./run-tests
cargo test --lib --target x86_64-unknown-linux-gnu
```

## TODOs

* Do on-device unit tests with [defmt-test](https://github.com/knurling-rs/defmt/tree/main/firmware/defmt-test)
