use std::{error::Error, fmt};

mod pwm;

pub use pwm::FanPwm;

/// power of fan
#[derive(Clone, Copy)]
pub struct FanPower(u8);

pub trait Fan {
    fn try_set_power(&mut self, power: FanPower) -> Result<(), Box<dyn Error>>;
}

impl From<u8> for FanPower {
    fn from(value: u8) -> Self {
        Self(value)
    }
}

impl FanPower {
    pub fn full_speed() -> Self {
        Self(255u8)
    }
}

impl fmt::Display for FanPower {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let precision = f.precision().unwrap_or(1);

        let power = if f.alternate() {
            let power = self.0;
            format!("FanPower({power})")
        } else {
            let power = (self.0 as f32) / 2.55;
            format!("{power:.precision$}%")
        };

        if let Some(width) = f.width() {
            let to_fill = width.checked_sub(power.len()).unwrap_or(0);
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
            let (left, right) = (vec![filler; left], vec![filler; right]);
            let string: String = left
                .into_iter()
                .chain(power.chars())
                .chain(right.into_iter())
                .collect();
            f.write_str(&string)
        } else {
            f.write_str(&power)
        }
    }
}
