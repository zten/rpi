use std::error::Error;

use dhatmini::{Orientation, ST7789V2};
use dhatmini::TearingEffect;
use display_interface::WriteOnlyDataCommand;
use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    prelude::*,
    text::Text,
};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayMs;
use embedded_text::alignment::HorizontalAlignment;
use embedded_text::style::{HeightMode, TextBoxStyleBuilder};
use embedded_text::TextBox;
use linux_embedded_hal::Delay;
use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
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

    loop {
        drawstatus(&mut display);
        Delay.delay_ms(1_000u16);
    }
}

fn drawtext<DI, RST>(mut display: &mut ST7789V2<DI, RST>)
    where DI: WriteOnlyDataCommand,
          RST: embedded_hal::digital::v2::OutputPin
{
    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::RED);

    display.clear(Rgb565::BLACK).unwrap_or_default();
    Text::new("Hello,\nRust!", Point::new(2, 28), style).draw(display).unwrap_or_default();
}

fn drawgraphics<DI, RST>(mut display: &mut ST7789V2<DI, RST>)
    where DI: WriteOnlyDataCommand,
          RST: embedded_hal::digital::v2::OutputPin
{
    let circle1 =
        Circle::new(Point::new(128, 64), 64).into_styled(PrimitiveStyle::with_fill(Rgb565::RED));
    let circle2 = Circle::new(Point::new(64, 64), 64)
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::GREEN, 1));

    let blue_with_red_outline = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::BLUE)
        .stroke_color(Rgb565::RED)
        .stroke_width(1) // > 1 is not currently supported in embedded-graphics on triangles
        .build();
    let triangle = Triangle::new(
        Point::new(40, 120),
        Point::new(40, 220),
        Point::new(140, 120),
    )
        .into_styled(blue_with_red_outline);

    let line = Line::new(Point::new(10, 10), Point::new(319, 239))
        .into_styled(PrimitiveStyle::with_stroke(RgbColor::WHITE, 10));


    // draw two circles on black background
    display.clear(Rgb565::BLACK).unwrap_or_default();
    circle1.draw(display).unwrap_or_default();
    circle2.draw(display).unwrap_or_default();
    triangle.draw(display).unwrap_or_default();
    line.draw(display).unwrap_or_default();
}

fn capture_output(cmd: &str) -> String {
    return match Exec::shell(cmd)
        .stdout(Redirection::Pipe)
        .capture() {
        Ok(capture) => { capture.stdout_str() }
        Err(_) => { String::new() }
    }
}

fn drawstatus<DI, RST>(mut display: &mut ST7789V2<DI, RST>)
    where DI: WriteOnlyDataCommand,
          RST: embedded_hal::digital::v2::OutputPin
{
    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::RED);
    let base_x = 2;
    let base_y = 28;

    display.clear(Rgb565::BLACK).unwrap_or_default();

    let character_style = MonoTextStyle::new(&FONT_10X20, Rgb565::RED);
    let textbox_style = TextBoxStyleBuilder::new()
        .height_mode(HeightMode::FitToText)
        .alignment(HorizontalAlignment::Justified)
        .paragraph_spacing(6)
        .build();


    let bounds = Rectangle::new(Point::zero(), Size::new(320, 0));

    let ip = capture_output("hostname -I | cut -d\' \' -f1");
    let cpu = capture_output("top -bn1 | grep load | awk '{printf \"CPU: %.2f\", $(NF-2)}'");
    let mem_usage = capture_output("free -m | awk 'NR==2{printf \"Mem: %s/%sMB %.2f%%\", $3,$2,$3*100/$2 }'");
    let disk_usage = capture_output("df -h | awk '$NF==\"/\"{printf \"Disk: %d/%dGB %s\", $3,$2,$5}'");
    let cpu_temp = capture_output("vcgencmd measure_temp |cut -f 2 -d '='");

    let text = format!("IP: {}\n{}   Temp: {}\nMem: {:>20}\nDisk: {:>20}", ip, cpu, cpu_temp, mem_usage, disk_usage);
    let text_box = TextBox::with_textbox_style(text.as_str(), bounds, character_style, textbox_style);

    text_box.draw(display).unwrap_or_default();

    // Text::new(ip.as_str(), Point::new(base_x, base_y), style).draw(display).unwrap_or_default();
    // Text::new(cpu.as_str(), Point::new(base_x, base_y + 32), style).draw(display).unwrap_or_default();
    // Text::new(cpu_temp.as_str(), Point::new(base_x + 144, base_y + 32), style).draw(display).unwrap_or_default();
    // Text::new(mem_usage.as_str(), Point::new(base_x, base_y + 62), style).draw(display).unwrap_or_default();
    // Text::new(disk_usage.as_str(), Point::new(base_x, base_y + 92), style).draw(display).unwrap_or_default();
}