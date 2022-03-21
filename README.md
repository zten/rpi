# rpi

Implements a ST7789V2 driver. I wanted to use a Display HAT Mini
from Pimoroni, and the initialization sequence in the ST7789 driver
didn't seem to work. So, I ported the Python commands and data.

If it was painfully obvious, I don't know Rust.

This library isn't published in Cargo and needs some work to rearrange
the directory structure to do so.

Uses MIT licensed software from:
* https://github.com/pimoroni/st7789-python
* https://github.com/almindor/st7789