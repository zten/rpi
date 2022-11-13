use std::error::Error;

use dhatmini::TearingEffect;
use dhatmini::{Orientation, ST7789V2};
use display_interface::WriteOnlyDataCommand;
use display_interface_spi::SPIInterfaceNoCS;
use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayMs;
use image::{DynamicImage, Rgb, RgbImage};
use linux_embedded_hal::Delay;
use rand;
use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use rsa::{
    pkcs8::DecodePublicKey, PaddingScheme, PublicKey, PublicKeyParts, RsaPrivateKey, RsaPublicKey,
};
use rusqlite::Connection;
use rusttype::{point, Font, Scale};
use std::collections::HashMap;
use std::env;
use std::str;
use std::thread;
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
    display
        .set_orientation(Orientation::LandscapeSwapped)
        .unwrap();
    display
        .set_tearing_effect(TearingEffect::HorizontalAndVertical)
        .unwrap();

    let font_data = include_bytes!("../PixelOperator.ttf");
    // This only succeeds if collection consists of one font
    let font = Font::try_from_bytes(font_data as &[u8]).expect("Error constructing Font");

    let vz_pw = env::var("VZ_PW").unwrap_or_default();
    let conn = Connection::open("5g_data.db")?;
    conn.execute(
        "create table if not exists vz_5g_status (
            id bigint primary key,
            ts datetime default current_timestamp,
            mode varchar(3) not null,
            signal integer not null,
            rsrp integer not null
        )
        ",
        [],
    )?;

    thread.spawn(move || {
        let conn = Connection::open("5g_data.db")?;

        loop {
            update_5g_info(&vz_pw, &conn);
            Delay.delay_ms(500u16);
        }
    });

    loop {
        drawstatus(&mut display, &font, &conn);
        Delay.delay_ms(1_000u16);
    }
}

fn update_5g_info(vz_pw: &str, x: &Connection) {
    if vz_pw.len() > 0 {
        let client = reqwest::blocking::Client::new();

        let public_key_req = client
            .get("http://192.168.0.1/cgi-bin/luci/verizon/sentPublicKey")
            .send();

        match public_key_req {
            Ok(resp) => {
                let mut rng = rand::thread_rng();

                let public_key = resp.text()?;
                let public_key_rsa = RsaPublicKey::from_public_key_pem(&public_key)?;

                let username = b"";
                let username_enc = public_key_rsa
                    .encrypt(rng, PaddingScheme::new_pkcs1v15_encrypt(), username)
                    .unwrap();

                let pw_enc = public_key_rsa
                    .encrypt(rng, PaddingScheme::new_pkcs1v15_encrypt(), vz_pw.as_bytes())
                    .unwrap();

                let login_json = format!(
                    "{{\"username\":\"{}\",\"password\":\"{}\"}}",
                    str::from_utf8(&username_enc).unwrap(),
                    str::from_utf8(&pw_enc).unwrap()
                );

                let login = client
                    .post("http://192.168.0.1/cgi-bin/luci/verizon")
                    .body("")
                    .send();

                match login {
                    Ok(resp) => {}
                    Err(_) => {}
                }
            }
            Err(_) => {}
        }
    } else {
    }
}

fn capture_output(cmd: &str) -> String {
    return match Exec::shell(cmd).stdout(Redirection::Pipe).capture() {
        Ok(capture) => capture.stdout_str(),
        Err(_) => String::new(),
    };
}

fn drawstatus<DI, RST>(display: &mut ST7789V2<DI, RST>, font: &Font, vz_pw: &Connection)
where
    DI: WriteOnlyDataCommand,
    RST: embedded_hal::digital::v2::OutputPin,
{
    let mut image = DynamicImage::new_rgb8(320, 240).to_rgb8();
    image.fill(0);
    let color = (255, 255, 255);

    let ip = format!("IP: {}", capture_output("hostname -I | cut -d\' \' -f1"));
    let ip_str = chomp(&ip);
    let cpu = format!(
        "{}%",
        capture_output("top -bn1 | grep load | awk '{printf \"CPU: %.2f\", $(NF-2)}'")
    );
    let mem_usage =
        capture_output("free -m | awk 'NR==2{printf \"Mem: %s/%sMB %.2f%%\", $3,$2,$3*100/$2 }'");
    let disk_usage =
        capture_output("df -h | awk '$NF==\"/\"{printf \"Disk: %d/%dGB %s\", $3,$2,$5}'");
    let cpu_temp = capture_output("vcgencmd measure_temp |cut -f 2 -d '='");
    let cpu_temp_str = chomp(&cpu_temp);
    let ssh_unchomped = format!("SSH users: {}", capture_output("who | wc -l"));
    let ssh_users = chomp(ssh_unchomped.as_str());

    draw_text(color, 0, 0, 32.0, &font, &mut image, ip_str);
    draw_text(color, 0, 28, 32.0, &font, &mut image, cpu.as_str());
    draw_text(color, 144, 28, 32.0, &font, &mut image, cpu_temp_str);
    draw_text(color, 0, 56, 32.0, &font, &mut image, mem_usage.as_str());
    draw_text(color, 0, 84, 32.0, &font, &mut image, disk_usage.as_str());
    draw_text(color, 0, 112, 32.0, &font, &mut image, ssh_users);

    draw_image(display, image);
}

fn chomp(s: &str) -> &str {
    return &s[..s.len() - 1];
}

fn draw_image<DI, RST>(display: &mut ST7789V2<DI, RST>, image: RgbImage)
where
    DI: WriteOnlyDataCommand,
    RST: embedded_hal::digital::v2::OutputPin,
{
    display
        .set_pixels(
            0,
            0,
            319,
            239,
            image.pixels().map(|pixel| {
                ((u16::from(pixel.0[0]) & 0xf8) << 8)
                    + ((u16::from(pixel.0[1]) & 0xfc) << 3)
                    + (u16::from(pixel.0[2]) >> 3)
            }),
        )
        .unwrap_or_default();
}

fn draw_text(
    color: (u8, u8, u8),
    start_x: u32,
    start_y: u32,
    font_size: f32,
    font: &Font,
    image: &mut RgbImage,
    text: &str,
) {
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
