use smart_leds::hsv::{hsv2rgb, Hsv};
use smart_leds::RGB;
use std::time::Duration;

/// Converts a given value in milliseconds to an RGB color value.
///
/// # Arguments
///
/// * `sample` - How long the ping took, or None for "timeout"
/// * `max` - Durations larger than this should be considered problems
///
/// # Returns
///
/// An RGB<u8> value representing the converted color.
pub fn ms2rgb(sample: Option<Duration>, max: Duration, brightness: u8) -> RGB<u8> {
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
            // >max: magenta
            if ms > max {
                Hsv {
                    hue: 210,
                    sat: 255,
                    val: brightness / 2,
                }
            }
            // 0-max: spectrum green(80)-yellow(40)-red(0)
            else {
                let frac = 1.0 - (ms as f32 / max as f32);
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

    const TEST_MAX: Duration = Duration::from_millis(100);

    #[test]
    fn timeout_returns_red() {
        assert_eq!(ms2rgb(None, TEST_MAX), RGB::new(255, 0, 0));
    }

    #[test]
    fn fast_returns_green() {
        assert_eq!(
            ms2rgb(Some(Duration::from_millis(0)), TEST_MAX),
            RGB::new(0, 255, 0)
        );
    }

    #[test]
    fn slow_returns_yellow() {
        assert_eq!(
            ms2rgb(Some(Duration::from_millis(50)), TEST_MAX),
            RGB::new(128, 255, 0)
        );
    }

    #[test]
    fn very_slow_returns_red() {
        assert_eq!(
            ms2rgb(Some(Duration::from_millis(200)), TEST_MAX),
            RGB::new(127, 0, 0)
        );
    }
}
