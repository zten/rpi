use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::str;
use std::thread;

use dhatmini::{Orientation, ST7789V2};
use dhatmini::TearingEffect;
use display_interface::WriteOnlyDataCommand;
use display_interface_spi::SPIInterfaceNoCS;
use embedded_hal::prelude::_embedded_hal_blocking_delay_DelayMs;
use image::{DynamicImage, Rgb, RgbImage};
use linux_embedded_hal::Delay;
use rand;
use reqwest::StatusCode;
use rppal::gpio::{Gpio, OutputPin};
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use rsa::{
    PaddingScheme, pkcs8::DecodePublicKey, PublicKey, PublicKeyParts, RsaPrivateKey, RsaPublicKey,
};
use rusqlite::Connection;
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

    let thread = thread::spawn(move || {
        let conn = Connection::open("5g_data.db").unwrap();

        loop {
            println!("running update on vz_5g_status");
            update_5g_info(&vz_pw, &conn);
            Delay.delay_ms(500u16);
        }
    });

    loop {
        drawstatus(&mut display, &font, &conn);
        Delay.delay_ms(1_000u16);
    }
}

struct Status {
    mode: String,
    signal: i32,
    rsrp: i32,
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Mode: {} Signal: {} RSRP: {}", self.mode, self.signal, self.rsrp)
    }
}

fn get_5g_status(db: &Connection) -> Result<Status, rusqlite::Error> {
    let mut stmt = db.prepare("select mode, signal, rsrp from vz_5g_status order by id desc limit 1")?;
    let mut rows = stmt.query([])?;
    rows.next()?.map(|row| {
        Ok(Status {
            mode: row.get(0)?,
            signal: row.get(1)?,
            rsrp: row.get(2)?,
        })
    }).unwrap_or(Err(rusqlite::Error::QueryReturnedNoRows))
}

fn update_5g_info(vz_pw: &str, db: &Connection) {
    if vz_pw.len() > 0 {
        let client = reqwest::blocking::ClientBuilder::new()
            .build()
            .unwrap();

        let public_key_req = client
            .get("http://192.168.0.1/cgi-bin/luci/verizon/sentPublicKey")
            .send();

        match public_key_req {
            Ok(resp) => {
                if resp.status() == StatusCode::OK {
                    let mut rng = rand::thread_rng();

                    let public_key = resp.text().unwrap()
                        .replace("\"", "")
                        .replace("\\n", "\n")
                        .replace("\\", "");

                    let public_key_rsa = RsaPublicKey::from_public_key_pem(&public_key).unwrap();

                    let username = b"";
                    let username_enc = public_key_rsa
                        .encrypt(&mut rng, PaddingScheme::new_pkcs1v15_encrypt(), username)
                        .unwrap();

                    let pw_enc = public_key_rsa
                        .encrypt(&mut rng, PaddingScheme::new_pkcs1v15_encrypt(), vz_pw.as_bytes())
                        .unwrap();

                    let login_json = format!(
                        "{{\"luci_username\":\"{}\",\"luci_password\":\"{}\"}}",
                        base64::encode_config(&username_enc, base64::STANDARD),
                        base64::encode_config(&pw_enc, base64::STANDARD)
                    );

                    let login = client
                        .post("http://192.168.0.1/cgi-bin/luci/verizon")
                        .body(login_json)
                        .send();

                    match login {
                        Ok(resp) => {
                            if resp.status() == StatusCode::FOUND {
                                let sysauth = resp
                                    .cookies()
                                    .find(|c| c.name() == "sysauth")
                                    .unwrap()
                                    .value()
                                    .to_string();

                                match client
                                    .get("http://192.168.0.1/cgi-bin/luci/verizon/network/getStatus")
                                    .header("Cookie", format!("sysauth={}", sysauth))
                                    .send() {
                                    Ok(resp) => {
                                        println!("status call successful");
                                        let status: HashMap<String, String> = resp.json().unwrap();

                                        let mode = status.get("modemtype").unwrap();
                                        let signal = status.get("signal").unwrap();
                                        let rsrp = status.get("rsrp").unwrap();

                                        db.execute(
                                            "insert into vz_5g_status (mode, signal, rsrp) values (?1, ?2, ?3)",
                                            [mode, signal, rsrp],
                                        ).unwrap();
                                    }
                                    _ => {
                                        println!("failed to get status");
                                    }
                                }
                            } else {
                                println!("login failed: {:?}", resp);
                            }
                        }
                        Err(_) => {
                            println!("Login failed");
                        }
                    }
                } else {
                    println!("Error getting public key: {}", resp.status());
                }
            }
            Err(_) => {
                println!("Failed to get public key");
            }
        }
    } else {
        println!("No password set");
    }
}

fn capture_output(cmd: &str) -> String {
    return match Exec::shell(cmd).stdout(Redirection::Pipe).capture() {
        Ok(capture) => capture.stdout_str(),
        Err(_) => String::new(),
    };
}

fn drawstatus<DI, RST>(display: &mut ST7789V2<DI, RST>, font: &Font, db: &Connection)
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
    let modem_status = get_5g_status(db).unwrap_or(Status {
        mode: "N/A".to_string(),
        signal: 0,
        rsrp: 0,
    });
    let modem_status_str = format!("{}", modem_status);

    draw_text(color, 0, 0, 28.0, &font, &mut image, ip_str);
    draw_text(color, 0, 28, 28.0, &font, &mut image, cpu.as_str());
    draw_text(color, 144, 28, 28.0, &font, &mut image, cpu_temp_str);
    draw_text(color, 0, 56, 28.0, &font, &mut image, mem_usage.as_str());
    draw_text(color, 0, 84, 28.0, &font, &mut image, disk_usage.as_str());
    draw_text(color, 0, 112, 28.0, &font, &mut image, ssh_users);
    draw_text(color, 0, 140, 28.0, &font, &mut image, modem_status_str.as_str());

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
                    let x = start_x + x + bounding_box.min.x as u32;
                    let y = start_y + y + bounding_box.min.y as u32;
                    if x < 320 && y < 240 {
                        image.put_pixel(x, y, Rgb([color.0, color.1, color.2]));
                    }
                }
            });
        }
    }
}
