use std::error::Error;
use std::io::Write;
use display_interface::WriteOnlyDataCommand;

use display_interface_spi::SPIInterfaceNoCS;
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    prelude::*,
    text::Text,
};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use linux_embedded_hal::Delay;
use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};

use dhatmini::{Orientation, ST7789V2};

mod dhatmini;

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
    display.set_orientation(Orientation::Landscape).unwrap();

    // drawgraphics(&mut display);
    drawtext(&mut display);

    println!("Rendering done");

    Ok(())
}

fn drawtext<DI, RST>(mut display: &mut ST7789V2<DI, RST>)
    where DI: WriteOnlyDataCommand,
          RST: embedded_hal::digital::v2::OutputPin
{
    let style = MonoTextStyle::new(&FONT_6X10, Rgb565::RED);

    display.clear(Rgb565::BLACK).unwrap_or_default();
    Text::new("Hello,\nRust!", Point::new(2, 28), style).draw(display).unwrap_or_default();
}

fn drawgraphics<DI, RST, PinE>(mut display: &mut ST7789V2<DI, RST>)
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

