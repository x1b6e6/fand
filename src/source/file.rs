use super::{Source, Temperature};
use std::{
    cell::RefCell,
    error::Error,
    fs::File,
    io::{self, Read as _, Seek as _, SeekFrom},
    path::{Path, PathBuf},
};

pub struct SourceFile {
    file_path: PathBuf,
    file: RefCell<Option<File>>,
    factor: f32,
}

fn file_read(file: &mut File, buf: &mut [u8]) -> io::Result<usize> {
    file.seek(SeekFrom::Start(0))?;
    file.read(buf)
}

impl SourceFile {
    pub fn new(path: impl AsRef<Path>, factor: Option<f32>) -> io::Result<Self> {
        let file_path = PathBuf::from(path.as_ref());
        let file = File::options().read(true).open(path)?;
        let file = RefCell::new(Some(file));
        let factor = factor.unwrap_or(0.001);

        Ok(Self {
            file_path,
            file,
            factor,
        })
    }

    fn file_read(&self, buf: &mut [u8]) -> io::Result<usize> {
        let mut file = self.file.borrow_mut();
        let file_ref = match file.as_mut() {
            Some(file) => file,
            None => {
                *file = Some(File::options().read(true).open(&self.file_path)?);
                unsafe { file.as_mut().unwrap_unchecked() }
            }
        };

        let ret = file_read(file_ref, buf);

        if ret.is_err() {
            *file = None;
        }
        ret
    }
}

impl Source for SourceFile {
    fn try_get_temperature(&self) -> Result<Temperature, Box<dyn Error>> {
        let mut buf = [0u8; 10];
        let size = self.file_read(&mut buf)?;
        let buf = unsafe { std::slice::from_raw_parts(buf.as_ptr(), size - 1) };
        let temp = unsafe { std::str::from_utf8_unchecked(&buf) };
        let temp: u32 = temp.parse()?;

        Ok(Temperature::from_celsius(temp as f32 * self.factor))
    }
}
