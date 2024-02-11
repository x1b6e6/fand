use super::{Source, Temperature};
use std::{
    cell::RefCell,
    error::Error,
    fs::File,
    io::{self, Read as _, Seek as _, SeekFrom},
    path::Path,
};

pub struct SourceFile {
    file: RefCell<File>,
    factor: f32,
}

impl SourceFile {
    pub fn new(path: impl AsRef<Path>, factor: Option<f32>) -> Result<Self, io::Error> {
        let file = File::options().read(true).open(path)?;
        let file = RefCell::new(file);
        let factor = factor.unwrap_or(0.001);

        Ok(Self { file, factor })
    }
}

impl Source for SourceFile {
    fn value(&self) -> Result<Temperature, Box<dyn Error>> {
        let mut buf = [0u8; 10];
        self.file.borrow_mut().seek(SeekFrom::Start(0))?;
        let size = self.file.borrow_mut().read(&mut buf)?;
        let buf = unsafe { std::slice::from_raw_parts(buf.as_ptr(), size - 1) };
        let temp = unsafe { std::str::from_utf8_unchecked(&buf) };
        let temp: u32 = temp.parse()?;

        Ok(Temperature::from_celsius(temp as f32 * self.factor))
    }
}
