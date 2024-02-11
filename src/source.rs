mod file;
mod nvidia;

use std::{error::Error, fmt};

pub use file::SourceFile;
pub use nvidia::SourceNvidia;

/// temperature
#[derive(Clone, Copy)]
pub struct Temperature(f32);

impl Temperature {
    pub fn from_celsius(value: f32) -> Self {
        Self(value)
    }

    pub fn celsius(self) -> f32 {
        self.0
    }
}

/// trait for access source of temperature
pub trait Source {
    fn try_get_temperature(&self) -> Result<Temperature, Box<dyn Error>>;
}

impl fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let precision = f.precision().unwrap_or(2);

        let temp = if f.alternate() {
            let value = self.0;
            format!("Temperature({value})")
        } else {
            let value = self.0;
            format!("{value:.precision$}°C")
        };

        if let Some(width) = f.width() {
            let to_fill = width.checked_sub(temp.chars().count()).unwrap_or(0);
            let (left, right) = match f.align().unwrap_or(fmt::Alignment::Right) {
                fmt::Alignment::Right => (to_fill, 0),
                fmt::Alignment::Center => {
                    if to_fill % 2 == 1 {
                        (1 + to_fill / 2, to_fill / 2)
                    } else {
                        (to_fill / 2, to_fill / 2)
                    }
                }
                fmt::Alignment::Left => (0, to_fill),
            };

            let filler = f.fill();
            let string: String = std::iter::repeat(filler)
                .take(left)
                .chain(temp.chars())
                .chain(std::iter::repeat(filler).take(right))
                .collect();
            f.write_str(&string)
        } else {
            f.write_str(&temp)
        }
    }
}
