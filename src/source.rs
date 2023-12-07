mod file;
mod nvidia;

use std::error::Error;

pub use file::SourceFile;
pub use nvidia::SourceNvidia;

/// temperature
pub struct Temperature(f32);

impl Temperature {
    pub fn from_celcius(value: f32) -> Self {
        Self(value)
    }

    pub fn celcius(self) -> f32 {
        self.0
    }
}

/// trait for access source of temperature
pub trait Source {
    fn value(&self) -> Result<Temperature, Box<dyn Error>>;
}
