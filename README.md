# pinetime-rs

Rust & [RTIC](https://rtic.rs/dev/book/en/) running on the PineTime watch.

See the [PineTime Wiki](https://wiki.pine64.org/index.php/PineTime) for docs.

A lot of this was inspired by [InfiniTime](https://github.com/JF002/InfiniTime).

```bash
cargo install probe-run cargo-embed flip-link
```

Run with `cargo run --release` or `cargo embed --release`.

## Simulator

See [pinetime-simulator](host-tools/pinetime-simulator) crate.

![pinetime_simulator.png](doc/pinetime_simulator.png)

## TODOs

* Redo linker scripts so image goes into the existing bootloader's firmware slot
  and use the bootloader update procedure, see [pinetime-mcuboot-bootloader](https://github.com/JF002/pinetime-mcuboot-bootloader)
* Figure out some shared-bus for SPIM0, used by the ST7789 and external SPI NOR flash for
  persistent storage/fs, maybe use [tickv](https://github.com/tock/tock/tree/master/libraries/tickv)
* Redo resource and priority management stuff
* Soft reset time persistent, something like [InfiniTime/pull/595](https://github.com/JF002/InfiniTime/pull/595)
* Impl low-power HAL stuff, see [nrf-hal/issues/279](https://github.com/nrf-rs/nrf-hal/issues/279)
