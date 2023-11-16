
Build:
======
Getting the esp toolchain running on an M1 mac was hours of hassle before I
gave up, so I tried dev containers, which are broken on my mac for personal
reasons. So I need to use vscode remote to ssh into my linux server, and then
use a dev container on the server.

Once the devcontainer is opened:

* `cargo build` to build a binary
* `F1 -> Start Wokwi` to start the binary in a simulator
* `WIFI_SSID=Foo WIFI_PASS=Bar cargo build` to do a build with wifi credentials
* `espflash flash --monitor /tmp/esp-ping-leds --port /dev/cu.usbmodem101` to flash to a device