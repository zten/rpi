use std::error::Error;

use dhatmini::{Orientation, ST7789V2};
use dhatmini::TearingEffect;
use display_interface::WriteOnlyDataCommand;
use display_interface_spi::SPIInterfaceNoCS;
use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayMs;
use image::{DynamicImage, Pixel, Rgb};
use linux_embedded_hal::Delay;
use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use rusttype::{Font, point, Scale};
use subprocess::{Exec, Redirection};

// from st7789-examples right now
fn main() -> Result<(), Box<dyn Error>> {
    let gpio = Gpio::new()?;
    let dc = gpio.get(9)?.into_output();
    let mut backlight = gpio.get(13)?.into_output();
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss1, 60_000_000, Mode::Mode0)?;
    let di = SPIInterfaceNoCS::new(spi, dc);
    // create driver
    let mut display = ST7789V2::new(di, None::<OutputPin>, 320, 240);

    // initialize
    display.init(Some(&mut backlight), &mut Delay).unwrap();
    // set default orientation
    display.set_orientation(Orientation::LandscapeSwapped).unwrap();
    display.set_tearing_effect(TearingEffect::HorizontalAndVertical).unwrap();

    let font_data = include_bytes!("../PixelOperator.ttf");
    // This only succeeds if collection consists of one font
    let font = Font::try_from_bytes(font_data as &[u8]).expect("Error constructing Font");

    loop {
        drawstatus(&mut display, &font);
        Delay.delay_ms(1_000u16);
    }
}

fn capture_output(cmd: &str) -> String {
    return match Exec::shell(cmd)
        .stdout(Redirection::Pipe)
        .capture() {
        Ok(capture) => { capture.stdout_str() }
        Err(_) => { String::new() }
    }
}

fn drawstatus<DI, RST>(display: &mut ST7789V2<DI, RST>, font: &Font)
    where DI: WriteOnlyDataCommand,
          RST: embedded_hal::digital::v2::OutputPin
{
    let mut image = DynamicImage::new_rgb8(320, 240).to_rgb8();
    image.fill(0);

    let ip = capture_output("hostname -I | cut -d\' \' -f1");
    let cpu = capture_output("top -bn1 | grep load | awk '{printf \"CPU: %.2f\", $(NF-2)}'");
    let mem_usage = capture_output("free -m | awk 'NR==2{printf \"Mem: %s/%sMB %.2f%%\", $3,$2,$3*100/$2 }'");
    let disk_usage = capture_output("df -h | awk '$NF==\"/\"{printf \"Disk: %d/%dGB %s\", $3,$2,$5}'");
    let cpu_temp = capture_output("vcgencmd measure_temp |cut -f 2 -d '='");

    let scale = Scale::uniform(12.0);
    let color = (255, 0, 0);
    let v_metrics = font.v_metrics(scale);
    let text = format!("IP: {}\n{}   Temp: {}\n{}\n{}", ip, cpu, cpu_temp, mem_usage, disk_usage);
    let glyphs: Vec<_> = font
        .layout(text.as_str(), scale, point(0.0, 0.0 + v_metrics.ascent))
        .collect();

    for glyph in glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            // Draw the glyph into the image per-pixel by using the draw closure
            glyph.draw(|x, y, v| {
                image.put_pixel(
                    // Offset the position by the glyph bounding box
                    x + bounding_box.min.x as u32,
                    y + bounding_box.min.y as u32,
                    Rgb([color.0, color.1, color.2]),
                )
            });
        }
    }

    display.set_pixels(0, 0, 319, 239,
                       image.pixels().map(|pixel| ((u16::from(pixel.0[0]) & 0xf8) << 8)
                           + (u16::from(pixel.0[1]) & 0xf3) << 3 + u16::from(pixel.0[2]) >> 3)).unwrap_or_default();
}