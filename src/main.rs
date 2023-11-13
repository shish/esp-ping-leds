use esp_idf_hal::{delay::FreeRtos, peripherals::Peripherals};
use esp_idf_svc::{
    eventloop::EspSystemEventLoop,
    nvs::EspDefaultNvsPartition,
    wifi::{AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi},
};
use esp_idf_sys as _;

const WIFI_SSID: &str = "Wokwi-GUEST";
const WIFI_PASS: &str = "";

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    loop {
        match fallible_main() {
            Ok(_) => unreachable!(),
            Err(e) => {
                log::error!("Major Error: {}", e);
                log::info!("Restarting in 10s...");
                FreeRtos::delay_ms(10000);
            }
        }
    }
}

fn fallible_main() -> anyhow::Result<()> {
    log::info!("Startup...");
    FreeRtos::delay_ms(1000);

    let peripherals = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    if false {
        log::info!("About to initialize WiFi (SSID: {})", WIFI_SSID);
        let mut wifi = BlockingWifi::wrap(
            EspWifi::new(peripherals.modem, sysloop.clone(), Some(nvs))?,
            sysloop,
        )?;
        connect_wifi(&mut wifi, WIFI_SSID, WIFI_PASS)?;
    }

    use smart_leds::hsv::{hsv2rgb, Hsv};
    use smart_leds::SmartLedsWrite;
    use ws2812_esp32_rmt_driver::Ws2812Esp32Rmt;
    let mut ws2812 = Ws2812Esp32Rmt::new(0, 9)?;
    //let mut ws2812_ = Ws2812Esp32Rmt::new(1, 9)?;

    let mut hue = 0; // unsafe { esp_random() } as u8;
    loop {
        // print!(".");
        let pixels = std::iter::repeat(hsv2rgb(Hsv {
            hue,
            sat: 255,
            val: 88,
        }))
        .take(25);
        ws2812.write(pixels.clone())?;
        //ws2812_.write(pixels)?;

        FreeRtos::delay_ms(100);

        hue = hue.wrapping_add(10);
    }
}

fn connect_wifi(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    wifi_ssid: &str,
    wifi_password: &str,
) -> anyhow::Result<()> {
    let wifi_configuration: Configuration = Configuration::Client(ClientConfiguration {
        ssid: wifi_ssid.into(),
        bssid: None,
        auth_method: AuthMethod::None,
        password: wifi_password.into(),
        channel: None,
    });

    wifi.set_configuration(&wifi_configuration)?;

    wifi.start()?;
    log::info!("Wifi started");

    wifi.connect()?;
    log::info!("Wifi connected");

    wifi.wait_netif_up()?;
    log::info!("Wifi netif up");

    Ok(())
}
