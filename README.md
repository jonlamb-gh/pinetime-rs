# pinetime-rs

Rust & [RTIC](https://rtic.rs/dev/book/en/) running on the PineTime watch.

See the [PineTime Wiki](https://wiki.pine64.org/index.php/PineTime) for docs.

```bash
cargo install probe-run cargo-embed flip-link
```

## TODOs

* Redo linker scripts so image goes into the existing bootloader's firmware slot and use the bootloader update procedure
* Do on-device unit tests with [defmt-test](https://github.com/knurling-rs/defmt/tree/main/firmware/defmt-test)
