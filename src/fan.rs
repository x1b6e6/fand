use std::error::Error;

mod pwm;

pub use pwm::FanPwm;

/// power of fan
#[derive(Clone, Copy)]
pub struct FanPower(u8);

pub trait Fan {
    fn write(&mut self, power: FanPower) -> Result<(), Box<dyn Error>>;
}

impl From<u8> for FanPower {
    fn from(value: u8) -> Self {
        Self(value)
    }
}
