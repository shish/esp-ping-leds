
Build:
======
Getting the esp toolchain running on an M1 mac was hours of hassle before I
gave up, so I tried dev containers, which are broken on my mac for personal
reasons. So I need to use vscode remote to ssh into my linux server, and then
use a dev container on the server.

Once the devcontainer is opened:

* `cargo build` to build a binary with default settings that work for a simulator
* `F1 -> Start Wokwi` to start the binary in a simulator
* `WIFI_SSID=Foo WIFI_PASS=Bar PING_HOST=1.1.1.1 cargo build` to do a build with wifi credentials and a specific host to ping (by default it will ping the local gateway)
* `espflash flash --monitor target/riscv32imc-esp-espidf/debug/esp-ping-leds --port /dev/cu.usbmodem101` to flash to a device (port depends on what board is being used - if that port doesn't work for you, drop the flag and espflash will scan to find all available ports)


To change the target board:
===========================
* `.devcontainer/devcontainer.json` - `build/args/ESP_BOARD`
* `rust-toolchain.toml` - `channel="esp"` for xtensa boards, `channel="nightly"` + `components = ["rust-src"]` for risc-v boards
* `.cargo/config.toml` - `build.target` and `board.MCU`
