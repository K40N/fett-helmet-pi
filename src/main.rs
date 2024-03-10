use std::{borrow::Cow, fs::File, io::Write, ops::DerefMut};

use anyhow::Result;

use png;

use serialport::{self, SerialPort};

const MCU_SERIAL_PORT: &'static str = "/dev/ttyUSB0";

fn main() -> Result<()> {
    let filename = "tallintest.png";
    println!("Opening png {}...", filename);
    let file = File::open(filename)?;
    println!("Reading png data...");
    let data = read_png(file)?;
    println!("Establishing connection with {}...", MCU_SERIAL_PORT);
    let mut mcu = HelmetMcu::new(MCU_SERIAL_PORT)?;
    println!("Constructing rotater...");
    let rot90 = Rot90::new(data, (64, 64));
    println!("Sending {} pixels...", 64 * 64);
    mcu.send(rot90)?;
    Ok(())
}

struct HelmetMcu<S: DerefMut<Target = T>, T: Write + ?Sized> {
    serial: S,
}

const RESET_SEQ: [u8; 11] = [b'#'; 11];

impl HelmetMcu<Box<dyn SerialPort>, dyn SerialPort> {
    fn new<'a>(serial_port_path: impl Into<Cow<'a, str>>) -> Result<Self> {
        Ok(
            Self {
                serial: serialport::new(
                    serial_port_path, 115200,
                ).open()?,
            }
        )
    }
}

impl<S: DerefMut<Target = T>, T: Write + ?Sized> HelmetMcu<S, T> {
    fn send(
        &mut self,
        data: impl Iterator<Item = u8>,
    ) -> Result<()> {
        self.serial.write_all(&RESET_SEQ)?;
        self.serial.flush()?;
        let mut index_within_row = -1;
        let mut byte = 0x0_u8;
        let mut index_within_byte = 0;
        for i in data {
            if i > (u8::MAX / 2) {
                byte |= 1 << index_within_byte;
            }
            index_within_byte += 1;
            if index_within_byte >= 8 {
                index_within_byte = 0;
                self.serial.write_all(&[byte])?;
                byte = 0x0_u8;
                index_within_row += 1;
                if index_within_row >= 8 {
                    index_within_row = -1;
                    self.serial.flush()?;
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
            }
            if index_within_row == -1 {
                self.serial.write_all(&[0x0])?;
                index_within_row += 1;
                continue;
            }
        }
        self.serial.flush()?;
        Ok(())
    }
}

fn read_png(file: File) -> Result<Vec<u8>> {
    let mut reader = png::Decoder::new(file).read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    reader.next_frame(&mut buf)?;
    Ok(buf)
}

struct Rot90<T: Copy> {
    orig: Vec<T>,
    w: usize,
    h: usize,
    x: usize,
    y: usize,
}

impl<T: Copy> Rot90<T> {
    fn new(orig: Vec<T>, dims: (usize, usize)) -> Self {
        let (w, h) = dims;
        assert!(orig.len() == (w * h));
        Self {
            orig, w, h,
            x: 0, y: 0,
        }
    }
}

impl<T: Copy> Rot90<T> {
    fn at_pre(&self, xt: usize, yt: usize) -> Option<<Self as Iterator>::Item> {
        let index = (yt * self.w) + xt;
        if index >= self.orig.len() {
            None
        } else {
            Some(self.orig[(yt * self.w) + xt])
        }
    }

    fn internal_peek(&self) -> Option<<Self as Iterator>::Item> {
        let xt = self.h - self.y;
        let yt = self.x;
        self.at_pre(xt, yt)
    }
}

impl<T: Copy> Iterator for Rot90<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let ret = self.internal_peek();
        if ret.is_some() {
            self.x += 1;
            if self.x >= self.h {
                self.x = 0;
                self.y += 1;
            }
        }
        ret
    }
} 
