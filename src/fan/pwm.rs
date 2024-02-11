use super::{Fan, FanPower};
use std::{
    error::Error,
    fs::File,
    io::{self, Read as _, Seek as _, Write as _},
    path::{Path, PathBuf},
};

struct PwmEnable {
    file: File,
    original: [u8; 4],
}

struct InnerFanPwm {
    file: File,
    _enable: PwmEnable,
}

pub struct FanPwm {
    pwm_path: PathBuf,
    pwm_enable_path: PathBuf,
    inner: Option<InnerFanPwm>,
}

fn file_write(file: &mut File, data: &[u8]) -> io::Result<()> {
    file.seek(io::SeekFrom::Start(0))?;
    file.write_all(data)?;
    file.flush()
}

impl FanPwm {
    pub fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let pwm_path = PathBuf::from(path.as_ref());
        let pwm_enable_path = PwmEnable::path_to_pwm_enable(path).unwrap();
        let inner = InnerFanPwm::new(&pwm_path, &pwm_enable_path)?;

        Ok(Self {
            pwm_path,
            pwm_enable_path,
            inner: Some(inner),
        })
    }

    fn file_write(&mut self, buf: &[u8]) -> io::Result<()> {
        let file = match &mut self.inner {
            Some(ref mut inner) => &mut inner.file,
            None => {
                self.inner = Some(InnerFanPwm::new(&self.pwm_path, &self.pwm_enable_path)?);

                unsafe { &mut self.inner.as_mut().unwrap_unchecked().file }
            }
        };

        let ret = file_write(file, buf);
        if ret.is_err() {
            self.inner.take();
        }
        ret
    }
}

impl InnerFanPwm {
    fn new(pwm: impl AsRef<Path>, pwm_enable: impl AsRef<Path>) -> io::Result<Self> {
        let file = File::options().write(true).truncate(true).open(&pwm)?;
        let enable = PwmEnable::new(pwm_enable)?;

        Ok(Self {
            file,
            _enable: enable,
        })
    }
}

impl PwmEnable {
    fn new(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut file = File::options().write(true).read(true).open(path)?;
        let mut original = [0u8; 4];

        file.read(&mut original)?;

        file_write(&mut file, &[0x31])?;

        Ok(Self { file, original })
    }

    fn path_to_pwm_enable(path: impl AsRef<Path>) -> Option<PathBuf> {
        let pwm_name =
            unsafe { std::str::from_utf8_unchecked(path.as_ref().file_name()?.as_encoded_bytes()) };

        Some(
            path.as_ref()
                .with_file_name(pwm_name.to_string() + "_enable"),
        )
    }
}

impl Drop for PwmEnable {
    fn drop(&mut self) {
        if let Err(e) = file_write(&mut self.file, &self.original) {
            log::error!("cannot disable pwm: {e}");
        }
    }
}

impl Fan for FanPwm {
    fn try_set_power(&mut self, power: FanPower) -> Result<(), Box<dyn Error>> {
        self.file_write(format!("{}", power.0).as_bytes())?;

        Ok(())
    }
}
