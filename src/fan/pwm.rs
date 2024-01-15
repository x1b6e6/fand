use super::{Fan, FanPower};
use std::{
    error::Error,
    fs::File,
    io::{self, Read, Seek as _, Write as _},
    path::{Path, PathBuf},
};

struct PwmEnable {
    file: File,
    original: [u8; 4],
}

pub struct FanPwm {
    file: File,
    _enable: PwmEnable,
}

fn file_write(file: &mut File, data: &[u8]) -> Result<(), io::Error> {
    file.seek(io::SeekFrom::Start(0))?;
    file.write(data)?;
    file.flush()
}

impl FanPwm {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let file = File::options().write(true).truncate(true).open(&path)?;
        let enable = PwmEnable::new(path)?;

        Ok(Self {
            file,
            _enable: enable,
        })
    }
}

impl PwmEnable {
    fn new(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let mut file = File::options()
            .write(true)
            .read(true)
            .open(Self::path_to_pwm_enable(path).unwrap())?;
        let mut original = [0u8; 4];

        file.read(&mut original)?;

        file_write(&mut file, &[0x31])?;

        Ok(Self { file, original })
    }

    fn path_to_pwm_enable(path: impl AsRef<Path>) -> Option<PathBuf> {
        let pwmname = std::str::from_utf8(path.as_ref().file_name()?.as_encoded_bytes()).ok()?;

        Some(
            path.as_ref()
                .with_file_name(pwmname.to_string() + "_enable"),
        )
    }
}

impl Drop for PwmEnable {
    fn drop(&mut self) {
        file_write(&mut self.file, &self.original).unwrap();
    }
}

impl Fan for FanPwm {
    fn write(&mut self, power: FanPower) -> Result<(), Box<dyn Error>> {
        file_write(&mut self.file, format!("{}", power.0).as_bytes())?;

        Ok(())
    }
}
