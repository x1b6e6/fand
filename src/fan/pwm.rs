use super::{Fan, FanPower};
use std::{
    error::Error,
    fs::File,
    io::{self, Seek as _, Write as _},
    path::Path,
};

pub struct FanPwm {
    file: File,
}

impl FanPwm {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let file = File::options().write(true).truncate(true).open(path)?;

        Ok(Self { file })
    }
}

impl Fan for FanPwm {
    fn write(&mut self, power: FanPower) -> Result<(), Box<dyn Error>> {
        self.file.seek(io::SeekFrom::Start(0))?;
        self.file.write_fmt(format_args!("{}", power.0))?;
        self.file.flush()?;

        Ok(())
    }
}
