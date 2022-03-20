#![no_std]
// associated re-typing not supported in rust yet
#![allow(clippy::type_complexity)]

//! This crate provides a ST7789 driver to connect to TFT displays.

mod instruction;

use instruction::Instruction;

use core::iter::once;

use display_interface::DataFormat::{U16BEIter, U8Iter};
use display_interface::WriteOnlyDataCommand;
use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::digital::v2::OutputPin;

#[cfg(feature = "graphics")]
mod graphics;

#[cfg(feature = "batch")]
mod batch;

///
/// ST7789 driver to connect to TFT displays.
///
pub struct ST7789<DI, RST>
    where
        DI: WriteOnlyDataCommand,
        RST: OutputPin,
{
    // Display interface
    di: DI,
    // Reset pin.
    rst: Option<RST>,
    // Visible size (x, y)
    size_x: u16,
    size_y: u16,
    // Current orientation
    orientation: Orientation,
}

///
/// Display orientation.
///
#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Orientation {
    Portrait = 0b0000_0000,         // no inverting
    Landscape = 0b0110_0000,        // invert column and page/column order
    PortraitSwapped = 0b1100_0000,  // invert page and column order
    LandscapeSwapped = 0b1010_0000, // invert page and page/column order
}

impl Default for Orientation {
    fn default() -> Self {
        Self::Portrait
    }
}

///
/// Tearing effect output setting.
///
#[derive(Copy, Clone)]
pub enum TearingEffect {
    /// Disable output.
    Off,
    /// Output vertical blanking information.
    Vertical,
    /// Output horizontal and vertical blanking information.
    HorizontalAndVertical,
}

///
/// An error holding its source (pins or SPI)
///
#[derive(Debug)]
pub enum Error<PinE> {
    DisplayError,
    Pin(PinE),
}

impl<DI, RST, PinE> ST7789<DI, RST>
    where
        DI: WriteOnlyDataCommand,
        RST: OutputPin<Error = PinE>,
{
    ///
    /// Creates a new ST7789 driver instance
    ///
    /// # Arguments
    ///
    /// * `di` - a display interface for talking with the display
    /// * `rst` - display hard reset pin
    /// * `size_x` - x axis resolution of the display in pixels
    /// * `size_y` - y axis resolution of the display in pixels
    ///
    pub fn new(di: DI, rst: Option<RST>, size_x: u16, size_y: u16) -> Self {
        Self {
            di,
            rst,
            size_x,
            size_y,
            orientation: Orientation::default(),
        }
    }

    ///
    /// Runs commands to initialize the display
    ///
    /// # Arguments
    ///
    /// * `delay_source` - mutable reference to a delay provider
    ///
    pub fn init(&mut self, delay_source: &mut impl DelayUs<u32>) -> Result<(), Error<PinE>> {
        self.hard_reset(delay_source)?;
        self.write_command(Instruction::SWRESET)?; // reset display
        delay_source.delay_us(150_000);
        self.write_command(Instruction::MADCTL)?;
        self.write_data(&[0x70])?;
        self.write_command(Instruction::FRMCTR2)?;
        self.write_data(&[0x0C, 0x0C, 0, 0x33, 0x33])?;
        self.write_command(Instruction::COLMOD)?;
        self.write_data(&[0x05])?;
        self.write_command(Instruction::GCTRL)?;
        self.write_data(&[0x14])?;
        self.write_command(Instruction::VCOMS)?;
        self.write_data(&[0x37])?;
        self.write_command(Instruction::LCMCTRL)?;
        self.write_data(&[0x2C])?;
        self.write_command(Instruction::VDVVRHEN)?;
        self.write_data(&[0x01])?;
        self.write_command(Instruction::VRHS)?;
        self.write_data(&[0x12])?;
        self.write_command(Instruction::VDVS)?;
        self.write_data(&[0x20])?;
        self.write_command(Instruction::PWCTRL1)?;
        self.write_data(&[0xA4, 0xA1])?;
        self.write_command(Instruction::FRCTRL2)?;
        self.write_data(&[0x0F])?;
        self.write_command(Instruction::GMCTRP1)?;
        self.write_data(&[0xD0, 0x04, 0x0D, 0x11, 0x13, 0x2B, 0x3F, 0x54,
            0x4C, 0x18, 0x0D, 0x0B, 0x1F, 0x23])?;
        self.write_command(Instruction::GMCTRN1)?;
        self.write_data(&[0xD0, 0x04, 0x0C, 0x11, 0x13, 0x2C, 0x3F, 0x44, 0x51, 0x2F,
            0x1F, 0x1F, 0x20, 0x23])?;
        self.write_command(Instruction::SLPOUT)?; // turn off sleep
        self.write_command(Instruction::DISPON)?; // turn on display
        delay_source.delay_us(10_000);
        Ok(())
    }

    ///
    /// Performs a hard reset using the RST pin sequence
    ///
    /// # Arguments
    ///
    /// * `delay_source` - mutable reference to a delay provider
    ///
    pub fn hard_reset(&mut self, delay_source: &mut impl DelayUs<u32>) -> Result<(), Error<PinE>> {
        match self.rst {
            Some(ref mut rst) => {
                rst.set_high().map_err(Error::Pin)?;
                delay_source.delay_us(10); // ensure the pin change will get registered
                rst.set_low().map_err(Error::Pin)?;
                delay_source.delay_us(10); // ensure the pin change will get registered
                rst.set_high().map_err(Error::Pin)?;
                delay_source.delay_us(10); // ensure the pin change will get registered
            }
            None => {}
        }

        Ok(())
    }

    ///
    /// Returns currently set orientation
    ///
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    ///
    /// Sets display orientation
    ///
    pub fn set_orientation(&mut self, orientation: Orientation) -> Result<(), Error<PinE>> {
        self.write_command(Instruction::MADCTL)?;
        self.write_data(&[orientation as u8])?;
        self.orientation = orientation;
        Ok(())
    }

    ///
    /// Sets a pixel color at the given coords.
    ///
    /// # Arguments
    ///
    /// * `x` - x coordinate
    /// * `y` - y coordinate
    /// * `color` - the Rgb565 color value
    ///
    pub fn set_pixel(&mut self, x: u16, y: u16, color: u16) -> Result<(), Error<PinE>> {
        self.set_address_window(x, y, x, y)?;
        self.write_command(Instruction::RAMWR)?;
        self.di
            .send_data(U16BEIter(&mut once(color)))
            .map_err(|_| Error::DisplayError)?;

        Ok(())
    }

    ///
    /// Sets pixel colors in given rectangle bounds.
    ///
    /// # Arguments
    ///
    /// * `sx` - x coordinate start
    /// * `sy` - y coordinate start
    /// * `ex` - x coordinate end
    /// * `ey` - y coordinate end
    /// * `colors` - anything that can provide `IntoIterator<Item = u16>` to iterate over pixel data
    ///
    pub fn set_pixels<T>(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
        colors: T,
    ) -> Result<(), Error<PinE>>
        where
            T: IntoIterator<Item = u16>,
    {
        self.set_address_window(sx, sy, ex, ey)?;
        self.write_command(Instruction::RAMWR)?;
        self.di
            .send_data(U16BEIter(&mut colors.into_iter()))
            .map_err(|_| Error::DisplayError)
    }

    ///
    /// Sets scroll offset "shifting" the displayed picture
    /// # Arguments
    ///
    /// * `offset` - scroll offset in pixels
    ///
    pub fn set_scroll_offset(&mut self, offset: u16) -> Result<(), Error<PinE>> {
        self.write_command(Instruction::VSCAD)?;
        self.write_data(&offset.to_be_bytes())
    }

    ///
    /// Release resources allocated to this driver back.
    /// This returns the display interface and the RST pin deconstructing the driver.
    ///
    pub fn release(self) -> (DI, Option<RST>) {
        (self.di, self.rst)
    }

    fn write_command(&mut self, command: Instruction) -> Result<(), Error<PinE>> {
        self.di
            .send_commands(U8Iter(&mut once(command as u8)))
            .map_err(|_| Error::DisplayError)?;
        Ok(())
    }

    fn write_data(&mut self, data: &[u8]) -> Result<(), Error<PinE>> {
        self.di
            .send_data(U8Iter(&mut data.iter().cloned()))
            .map_err(|_| Error::DisplayError)
    }

    // Sets the address window for the display.
    fn set_address_window(
        &mut self,
        sx: u16,
        sy: u16,
        ex: u16,
        ey: u16,
    ) -> Result<(), Error<PinE>> {
        self.write_command(Instruction::CASET)?;
        self.write_data(&sx.to_be_bytes())?;
        self.write_data(&ex.to_be_bytes())?;
        self.write_command(Instruction::RASET)?;
        self.write_data(&sy.to_be_bytes())?;
        self.write_data(&ey.to_be_bytes())
    }

    ///
    /// Configures the tearing effect output.
    ///
    pub fn set_tearing_effect(&mut self, tearing_effect: TearingEffect) -> Result<(), Error<PinE>> {
        match tearing_effect {
            TearingEffect::Off => self.write_command(Instruction::TEOFF),
            TearingEffect::Vertical => {
                self.write_command(Instruction::TEON)?;
                self.write_data(&[0])
            }
            TearingEffect::HorizontalAndVertical => {
                self.write_command(Instruction::TEON)?;
                self.write_data(&[1])
            }
        }
    }
}
