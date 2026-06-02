use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::RGB;
use std::time::Duration;

/// Converts a given value in milliseconds to an RGB color value.
///
/// # Arguments
///
/// * `sample` - How long the ping took, or None for "timeout"
/// * `min` - Durations less than this are perfectly healthy (solid green)
/// * `max` - Durations larger than this should be considered problems
///
/// # Returns
///
/// An RGB<u8> value representing the converted color.
pub fn ms2rgb(sample: Option<Duration>, min: Duration, max: Duration, brightness: u8) -> RGB<u8> {
    let min = min.as_millis() as u32;
    let max = max.as_millis() as u32;
    let hsv = match sample {
        // offline: dark blue
        None => Hsv {
            hue: 170,
            sat: 255,
            val: brightness / 2,
        },
        Some(d) => {
            let ms = d.as_millis() as u32;
            // <min: solid green
            if ms < min {
                Hsv {
                    hue: 80,
                    sat: 255,
                    val: brightness / 2,
                }
            }
            // >max: magenta
            else if ms > max {
                Hsv {
                    hue: 210,
                    sat: 255,
                    val: brightness / 2,
                }
            }
            // min-max: spectrum green(80)-yellow(40)-red(0)
            else {
                let frac = 1.0 - ((ms - min) as f32 / (max - min) as f32);
                Hsv {
                    hue: (80.0 * frac) as u8,
                    sat: 255,
                    val: brightness / 2,
                }
            }
        }
    };

    hsv2rgb(hsv)
}

#[cfg(test)]
mod test_ms2rgb {
    use super::*;

    #[allow(dead_code)]
    const TEST_MIN: Duration = Duration::from_millis(10);
    #[allow(dead_code)]
    const TEST_MAX: Duration = Duration::from_millis(100);
    #[allow(dead_code)]
    const TEST_BRIGHTNESS: u8 = 127;

    #[test]
    fn timeout_returns_dark_blue() {
        let result = ms2rgb(None, TEST_MIN, TEST_MAX, TEST_BRIGHTNESS);
        // Dark blue with brightness/2
        assert_eq!(result.b, TEST_BRIGHTNESS / 2);
    }

    #[test]
    fn very_fast_returns_solid_green() {
        let result = ms2rgb(
            Some(Duration::from_millis(5)),
            TEST_MIN,
            TEST_MAX,
            TEST_BRIGHTNESS,
        );
        // Solid green hue (80) at brightness/2
        assert!(result.g > 0);
    }

    #[test]
    fn fast_returns_green() {
        let result = ms2rgb(
            Some(Duration::from_millis(10)),
            TEST_MIN,
            TEST_MAX,
            TEST_BRIGHTNESS,
        );
        // Green hue (80) at brightness/2
        assert!(result.g > 0);
    }

    #[test]
    fn slow_returns_yellow_red() {
        let result = ms2rgb(
            Some(Duration::from_millis(50)),
            TEST_MIN,
            TEST_MAX,
            TEST_BRIGHTNESS,
        );
        // Yellow-ish (between green and red)
        assert!(result.r > 0 || result.g > 0);
    }

    #[test]
    fn very_slow_returns_magenta() {
        let result = ms2rgb(
            Some(Duration::from_millis(200)),
            TEST_MIN,
            TEST_MAX,
            TEST_BRIGHTNESS,
        );
        // Magenta with brightness/2
        assert_eq!(result.b, TEST_BRIGHTNESS / 2);
    }
}
