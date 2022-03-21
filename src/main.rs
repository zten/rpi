mod dhatmini;

use std::{thread, time};
use std::error::Error;

use display_interface_spi::{SPIInterface, SPIInterfaceNoCS};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::*;
use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use dhatmini::{Orientation, ST7789};
use linux_embedded_hal::Delay;

// from st7789-examples right now
fn main() -> Result<(), Box<dyn Error>> {
    let gpio = Gpio::new()?;
    let dc = gpio.get(9)?.into_output();
    let mut backlight = gpio.get(13)?.into_output();
    backlight.set_low();
    thread::sleep(time::Duration::from_millis(100));
    backlight.set_high();

    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss1, 60_000_000, Mode::Mode0)?;
    let di = SPIInterfaceNoCS::new(spi, dc);

    // create driver
    let mut display = ST7789::new(di, None::<OutputPin>, 320, 240);

    // initialize
    display.init(&mut Delay).unwrap();
    // set default orientation
    display.set_orientation(Orientation::PortraitSwapped).unwrap();

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
    display.clear(Rgb565::BLACK).unwrap();
    circle1.draw(&mut display).unwrap();
    circle2.draw(&mut display).unwrap();
    triangle.draw(&mut display).unwrap();
    line.draw(&mut display).unwrap();

    println!("Rendering done");

    Ok(())
}
