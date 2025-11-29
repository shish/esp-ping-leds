What:
=====
A lil' esp32 project to monitor my internet connection and teach myself embedded rust

Every minute, it pings a given IP address, and lights up one LED on a scale of green-to-red based on how long the ping was (with dark blue for "no data yet" and purple for "packet lost")

![Wooden V1](./.github/images/wooden.jpeg?raw=true)
![LEDs](./.github/images/leds.jpeg?raw=true)
![Glow](./.github/images/glow.jpeg?raw=true)


Build:
======
Getting the esp toolchain running on an M1 mac was hours of hassle before I gave up, but devcontainers seem to work \o/

Once the devcontainer is opened:

* `cargo build` to build a binary with default settings that work for a simulator
* `F1 -> Wokwi: Start Simulator` to start the binary in a simulator
* `WIFI_SSID=Foo WIFI_PASS=Bar PING_HOST=1.1.1.1 cargo build` to do a build with wifi credentials and a specific host to ping (by default it will ping the local gateway)
* `espflash flash --monitor target/riscv32imc-esp-espidf/debug/esp-ping-leds --port /dev/cu.usbmodem101` to flash to a device (port depends on what board is being used - if that port doesn't work for you, drop the flag and espflash will scan to find all available ports)
