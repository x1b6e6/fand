use super::{Source, Temperature};
use dlopen::wrapper::{Container, WrapperApi};
use log::error;
use std::{error::Error, ffi::CStr, fmt, mem::MaybeUninit, num::NonZeroI32};

#[derive(Clone, Copy)]
#[repr(C)]
struct NvidiaDeviceHandle(*const ());

#[derive(WrapperApi)]
pub struct NvidiaApi {
    #[dlopen_name = "nvmlInit_v2"]
    init: fn() -> Result<(), NvidiaError>,

    #[dlopen_name = "nvmlShutdown"]
    deinit: fn() -> Result<(), NvidiaError>,

    #[dlopen_name = "nvmlErrorString"]
    error_string: fn(ret: NvidiaError) -> *const u8,

    #[dlopen_name = "nvmlDeviceGetCount_v2"]
    devices_count: fn(count: &mut u32) -> Result<(), NvidiaError>,

    #[dlopen_name = "nvmlDeviceGetHandleByIndex_v2"]
    device_handle_by_index: fn(index: u32, dev: &mut NvidiaDeviceHandle) -> Result<(), NvidiaError>,

    #[dlopen_name = "nvmlDeviceGetBoardId"]
    device_get_board_id: fn(dev: NvidiaDeviceHandle, id: &mut u32) -> Result<(), NvidiaError>,

    #[dlopen_name = "nvmlDeviceGetName"]
    device_get_name:
        fn(dev: NvidiaDeviceHandle, name: *mut u8, len: u32) -> Result<(), NvidiaError>,

    #[dlopen_name = "nvmlDeviceGetTemperature"]
    device_get_temperature:
        fn(dev: NvidiaDeviceHandle, types: i32, temp: &mut u32) -> Result<(), NvidiaError>,
}

struct Nvidia {
    api: Container<NvidiaApi>,
    devices: Vec<NvidiaDeviceHandle>,
}

fn nvidia() -> &'static Nvidia {
    static mut NVIDIA: (bool, MaybeUninit<Nvidia>) = (false, MaybeUninit::uninit());

    let nvidia = unsafe { NVIDIA.1.assume_init_mut() };

    if unsafe { !NVIDIA.0 } {
        let api: Container<NvidiaApi> =
            unsafe { Container::load("libnvidia-ml.so") }.expect("loading libnvidia-ml.so");

        api.init().expect("init nvidia backend");

        unsafe {
            NVIDIA.0 = true;
            NVIDIA.1.write(Nvidia {
                api: api,
                devices: Vec::new(),
            })
        };

        let mut cnt = 0;
        nvidia
            .api
            .devices_count(&mut cnt)
            .expect("get nvidia device count");

        for index in 0..cnt {
            let mut handle = NvidiaDeviceHandle::null();
            nvidia
                .api
                .device_handle_by_index(index, &mut handle)
                .expect("create device handle");

            nvidia.devices.push(handle);
        }
    }

    return nvidia;
}

pub struct SourceNvidia {
    dev: NvidiaDeviceHandle,
}

#[derive(Clone, Copy)]
pub struct NvidiaError(NonZeroI32);

#[derive(Debug)]
pub enum SourceNvidiaError {
    NoDevices,
    NotFound {
        name: Option<String>,
        index: Option<u32>,
    },
    Error(NvidiaError),
}

impl Nvidia {
    fn try_find_device<E>(
        &self,
        filter: impl Fn(&NvidiaDeviceHandle) -> Result<bool, E>,
    ) -> Result<Option<&NvidiaDeviceHandle>, E> {
        for handle in self.devices.iter() {
            if filter(handle)? {
                return Ok(Some(handle));
            }
        }

        Ok(None)
    }
}

impl Drop for Nvidia {
    fn drop(&mut self) {
        if let Err(err) = self.api.deinit() {
            error!("cannot deinit nvidia api: {err:?}");
        }
    }
}

impl SourceNvidia {
    pub fn new(name: Option<String>, index: Option<u32>) -> Result<Self, SourceNvidiaError> {
        let nvidia = nvidia();

        let dev = match (&name, &index) {
            (Some(name), Some(index)) => nvidia
                .try_find_device(|dev| {
                    Ok::<_, NvidiaError>(dev.name()? == name.as_str() && dev.board_id()? == *index)
                })?
                .ok_or_else(|| SourceNvidiaError::NotFound {
                    name: Some(name.clone()),
                    index: Some(*index),
                }),
            (None, Some(index)) => nvidia
                .try_find_device(|dev| Ok::<_, NvidiaError>(dev.board_id()? == *index))?
                .ok_or_else(|| SourceNvidiaError::NotFound {
                    name: None,
                    index: Some(*index),
                }),
            (Some(name), None) => nvidia
                .try_find_device(|dev| Ok::<_, NvidiaError>(dev.name()? == name.as_str()))?
                .ok_or_else(|| SourceNvidiaError::NotFound {
                    name: Some(name.clone()),
                    index: None,
                }),
            (None, None) => nvidia.devices.first().ok_or(SourceNvidiaError::NoDevices),
        };

        let dev = *dev?;

        Ok(Self { dev })
    }
}

impl Source for SourceNvidia {
    fn value(&self) -> Result<Temperature, Box<dyn Error>> {
        let temp = self.dev.temp()?;
        Ok(Temperature::from_celsius(temp as f32))
    }
}

impl NvidiaDeviceHandle {
    fn null() -> Self {
        Self(std::ptr::null())
    }

    fn name(&self) -> Result<String, NvidiaError> {
        let mut buf = [0u8; 4096];
        let api = &nvidia().api;
        api.device_get_name(*self, buf.as_mut_ptr(), 4096)?;

        let name = unsafe { CStr::from_ptr(buf.as_ptr().cast()) };
        let name = unsafe { std::str::from_utf8_unchecked(name.to_bytes()) };

        Ok(name.to_string())
    }

    fn board_id(&self) -> Result<u32, NvidiaError> {
        let mut id = 0;
        let api = &nvidia().api;
        api.device_get_board_id(*self, &mut id)?;

        Ok(id)
    }

    fn temp(&self) -> Result<u32, NvidiaError> {
        let mut temp = 0;
        let api = &nvidia().api;
        api.device_get_temperature(*self, 0, &mut temp)?;

        Ok(temp)
    }
}

impl fmt::Display for NvidiaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let api = &nvidia().api;
        let err = api.error_string(*self);
        let err = unsafe { CStr::from_ptr(err.cast()) };
        let message = unsafe { std::str::from_utf8_unchecked(err.to_bytes()) };

        f.write_str(message)
    }
}

impl fmt::Debug for NvidiaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl Error for NvidiaError {}

impl From<NvidiaError> for SourceNvidiaError {
    fn from(value: NvidiaError) -> Self {
        Self::Error(value)
    }
}
