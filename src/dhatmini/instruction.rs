/// ST7789 instructions.
#[repr(u8)]
pub enum Instruction {
    SWRESET = 0x01,
    SLPOUT = 0x11,
    INVON = 0x21,
    DISPON = 0x29,
    CASET = 0x2A,
    RASET = 0x2B,
    RAMWR = 0x2C,
    MADCTL = 0x36,
    COLMOD = 0x3A,
    FRMCTR2 = 0xB2,
    GCTRL = 0xB7,
    VCOMS = 0xBB,
    LCMCTRL = 0xC0,
    VDVVRHEN = 0xC2,
    VRHS = 0xC3,
    VDVS = 0xC4,
    FRCTRL2 = 0xC6,
    PWCTRL1 = 0xD0,
    GMCTRP1 = 0xE0,
    GMCTRN1 = 0xE1,
    TEOFF = 0x34,
    TEON = 0x35,
    VSCAD = 0x37
}
