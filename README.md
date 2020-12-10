# stm32f411-rs-i2s-pac-pcm5102

This rust project is an example of i2s usage on a stm32f411 chip. It use this
protocol to output a sawtooth on a pcm5102 module. **Warning!** Reduce the
volume, the sawtooth is fullscale so it's very loud!

At time of this project, there is no Hardware Abstraction Layer for i2s.
Instead, the main source directly access to the hadware using `pac` module,
this is why it contains many unsafe sections.

This project is for a nucleo-f411re but should be easily adaptable for any
stm32f4xx using a 8Mhz external oscillator. To Adapt it, you need:
 - Change ram an flash in `memory.x` according to your device.
 - In `Cargo.toml`, under `dependencies.stm32f4xx-hal` replace the `stm32f411`
   feature according to your chip (See
[here](https://crates.io/crates/stm32f4xx-hal) for available chips).
 - In `Embed.toml`, under `default.general`, replace `chip` value by your chip
   (`cargo embed --list-chip` to get the list of possible value).

## Connecting devices

| pcm5102 | stm32f4xx  |
|---------|------------|
| VIN     | 3V3        |
| GND     | GND        |
| LCK     | PB12       |
| DIN     | PB15       |
| BCK     | PB13       |
| SCK     | PC6        |
| FLT     | 3V3 or GND |
| DEMP    | GND        |
| XSMP    | 3V3        |
| FMT     | GND        |
| A3V3    | 3V3        |
| AGND    | GND        |


## Build, load, and run

This require `cargo embed`, you can install it with `cargo install cargo-embed`.
Just run `cargo embed` to build, load, and run the firmware.

## License

This project is licensed under terms of both MIT and Apache licenses. See
[LICENSE-APACHE.txt](LICENSE-APACHE.txt) and
[LICENSE-MIT.txt](LICENSE-MIT.txt).
