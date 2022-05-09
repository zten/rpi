use std::error::Error;

use dhatmini::{Orientation, ST7789V2};
use dhatmini::TearingEffect;
use display_interface::WriteOnlyDataCommand;
use display_interface_spi::SPIInterfaceNoCS;
use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayMs;
use image::{DynamicImage, Rgb, RgbImage};
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
    };
}

fn drawstatus<DI, RST>(display: &mut ST7789V2<DI, RST>, font: &Font)
    where DI: WriteOnlyDataCommand,
          RST: embedded_hal::digital::v2::OutputPin
{
    let mut image = DynamicImage::new_rgb8(320, 240).to_rgb8();
    image.fill(0);
    let color = (255, 255, 255);

    let ip = format!("IP: {}", capture_output("hostname -I | cut -d\' \' -f1")).replace('\n', "");
    let cpu = format!("{}%", capture_output("top -bn1 | grep load | awk '{printf \"CPU: %.2f\", $(NF-2)}'")).replace('\n', "");
    let mem_usage = capture_output("free -m | awk 'NR==2{printf \"Mem: %s/%sMB %.2f%%\", $3,$2,$3*100/$2 }'");
    let disk_usage = capture_output("df -h | awk '$NF==\"/\"{printf \"Disk: %d/%dGB %s\", $3,$2,$5}'");
    let cpu_temp = capture_output("vcgencmd measure_temp |cut -f 2 -d '='");

    draw_text(color, 0, 0, 32.0, &font, &mut image, ip.as_str());
    draw_text(color, 0, 28, 32.0, &font, &mut image, cpu.as_str());
    draw_text(color, 144, 28, 32.0, &font, &mut image, cpu_temp.as_str());
    draw_text(color, 0, 56, 32.0, &font, &mut image, mem_usage.as_str());
    draw_text(color, 0, 84, 32.0, &font, &mut image, disk_usage.as_str());

    draw_image(display, image);
}

fn draw_image<DI, RST>(display: &mut ST7789V2<DI, RST>, image: RgbImage)
    where DI: WriteOnlyDataCommand,
          RST: embedded_hal::digital::v2::OutputPin
{
    display.set_pixels(0, 0, 319, 239,
                       image.pixels().map(|pixel| ((u16::from(pixel.0[0]) & 0xf8) << 8)
                           + (u16::from(pixel.0[1]) & 0xfc) << 3 + u16::from(pixel.0[2]) >> 3)).unwrap_or_default();
}

fn draw_text(color: (u8, u8, u8), start_x: u32, start_y: u32, font_size: f32, font: &Font, image: &mut RgbImage, text: &str) {
    let scale = Scale::uniform(font_size);
    let v_metrics = font.v_metrics(scale);

    let glyphs: Vec<_> = font
        .layout(text, scale, point(0.0, 0.0 + v_metrics.ascent))
        .collect();

    for glyph in glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            // Draw the glyph into the image per-pixel by using the draw closure
            glyph.draw(|x, y, v| {
                if v > 0 as f32 {
                    image.put_pixel(
                        // Offset the position by the glyph bounding box
                        start_x + x + bounding_box.min.x as u32,
                        start_y + y + bounding_box.min.y as u32,
                        Rgb([color.0, color.1, color.2]),
                    )
                }
            });
        }
    }
}