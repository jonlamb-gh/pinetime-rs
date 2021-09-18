# pinetime-rs

See [PineTime Wiki](https://wiki.pine64.org/index.php/PineTime).

```bash
cargo install probe-run cargo-embed flip-link
```

## Tests

```bash
# ./run-tests
cargo test --lib --target x86_64-unknown-linux-gnu
```

## TODOs

* Switch to on-device unit tests with [defmt-test](https://github.com/knurling-rs/defmt/tree/main/firmware/defmt-test)
  - see https://github.com/JF002/InfiniTime/blob/136d4bb85e36777f0f9877fd065476ba1c02ca90/src/FreeRTOS/port_cmsis_systick.c
