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
}

impl SourceFile {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let file = File::options().read(true).open(path)?;
        let file = RefCell::new(file);

        Ok(Self { file })
    }
}

impl Source for SourceFile {
    fn value(&self) -> Result<Temperature, Box<dyn Error>> {
        let mut buf = Vec::new();
        self.file.borrow_mut().seek(SeekFrom::Start(0))?;
        self.file.borrow_mut().read_to_end(&mut buf)?;
        let temp = unsafe { std::str::from_utf8_unchecked(&buf[..buf.len() - 1]) };
        let temp: u32 = temp.parse()?;

        Ok(Temperature::from_celcius(temp as f32 / 1000.0))
    }
}
