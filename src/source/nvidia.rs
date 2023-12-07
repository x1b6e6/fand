use super::{Source, Temperature};
use dlopen::wrapper::{Container, WrapperApi};
use log::error;
use std::{error::Error, ffi::CStr, mem::MaybeUninit, str::FromStr};

#[derive(Clone, Copy)]
#[repr(C)]
struct NvidiaDeviceHandle(*const ());

#[derive(WrapperApi)]
pub struct NvidiaApi {
    #[dlopen_name = "nvmlInit_v2"]
    init: fn() -> i32,

    #[dlopen_name = "nvmlShutdown"]
    deinit: fn() -> i32,

    #[dlopen_name = "nvmlErrorString"]
    error_string: fn(ret: i32) -> *const u8,

    #[dlopen_name = "nvmlDeviceGetCount_v2"]
    devices_count: fn(count: &mut u32) -> i32,

    #[dlopen_name = "nvmlDeviceGetHandleByIndex_v2"]
    device_handle_by_index: fn(index: u32, dev: &mut NvidiaDeviceHandle) -> i32,

    #[dlopen_name = "nvmlDeviceGetBoardId"]
    device_get_board_id: fn(dev: NvidiaDeviceHandle, id: &mut u32) -> i32,

    #[dlopen_name = "nvmlDeviceGetName"]
    device_get_name: fn(dev: NvidiaDeviceHandle, name: *mut u8, len: u32) -> i32,

    #[dlopen_name = "nvmlDeviceGetTemperature"]
    device_get_temperature: fn(dev: NvidiaDeviceHandle, types: i32, temp: &mut u32) -> i32,
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
            unsafe { Container::load("libnvidia-ml.so") }.expect("load libnvidia-ml.so");

        let ret = api.init();
        if ret != 0 {
            panic!("{:?}", NvidiaError::from_nvidia(&api, ret));
        }
        unsafe {
            NVIDIA.0 = true;
            NVIDIA.1.write(Nvidia {
                api: api,
                devices: Vec::new(),
            })
        };

        let mut cnt = 0;
        let ret = nvidia.api.devices_count(&mut cnt);
        if ret != 0 {
            panic!("{:?}", NvidiaError::from_nvidia(&nvidia.api, ret));
        }

        for index in 0..cnt {
            let mut handle = NvidiaDeviceHandle::null();
            let ret = nvidia.api.device_handle_by_index(index, &mut handle);
            if ret != 0 {
                panic!("{:?}", NvidiaError::from_nvidia(&nvidia.api, ret));
            }

            nvidia.devices.push(handle);
        }
    }

    return nvidia;
}

pub struct SourceNvidia {
    dev: NvidiaDeviceHandle,
}

#[derive(Debug)]
pub struct NvidiaError {
    message: String,
}

#[derive(Debug)]
pub enum SourceNvidiaError {
    NoDevices,
    NotFound {
        name: Option<String>,
        index: Option<u32>,
    },
}

impl Nvidia {
    fn find_device(
        &self,
        filter: impl Fn(&NvidiaDeviceHandle) -> bool,
    ) -> Option<&NvidiaDeviceHandle> {
        self.devices.iter().find(|dev| filter(*dev))
    }
}

impl Drop for Nvidia {
    fn drop(&mut self) {
        let ret = self.api.deinit();
        if ret != 0 {
            error!("{:?}", self.api.error_string(ret));
        }
    }
}

impl SourceNvidia {
    pub fn new(name: Option<String>, index: Option<u32>) -> Result<Self, SourceNvidiaError> {
        let nvidia = nvidia();

        let dev = match (&name, &index) {
            (Some(name), Some(index)) => nvidia
                .find_device(|dev| {
                    dev.name().unwrap() == name.as_str() && dev.board_id().unwrap() == *index
                })
                .ok_or(SourceNvidiaError::NoDevices),
            (None, Some(index)) => nvidia
                .find_device(|dev| dev.board_id().unwrap() == *index)
                .ok_or_else(|| SourceNvidiaError::NotFound {
                    name: None,
                    index: Some(*index),
                }),
            (Some(name), None) => nvidia
                .find_device(|dev| dev.name().unwrap() == name.as_str())
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
        Ok(Temperature::from_celcius(temp as f32))
    }
}

impl NvidiaError {
    fn from_nvidia(api: &Container<NvidiaApi>, ret: i32) -> Self {
        let err = api.error_string(ret);
        let err = unsafe { CStr::from_ptr(err as *const i8) };
        Self {
            message: String::from_str(err.to_str().unwrap()).unwrap(),
        }
    }
}

impl NvidiaDeviceHandle {
    fn null() -> Self {
        Self(std::ptr::null())
    }

    fn name(&self) -> Result<String, NvidiaError> {
        let mut buf = [0u8; 4096];
        let api = &nvidia().api;
        let ret = api.device_get_name(*self, &mut buf as *mut u8, 4096);
        if ret != 0 {
            return Err(NvidiaError::from_nvidia(api, ret));
        }

        let name = CStr::from_bytes_until_nul(&buf).unwrap();

        Ok(String::from_str(name.to_str().unwrap()).unwrap())
    }

    fn board_id(&self) -> Result<u32, NvidiaError> {
        let mut id = 0;
        let api = &nvidia().api;
        let ret = api.device_get_board_id(*self, &mut id);
        if ret != 0 {
            return Err(NvidiaError::from_nvidia(api, ret));
        }

        Ok(id)
    }

    fn temp(&self) -> Result<u32, NvidiaError> {
        let mut temp = 0;
        let api = &nvidia().api;
        let ret = api.device_get_temperature(*self, 0, &mut temp);
        if ret != 0 {
            return Err(NvidiaError::from_nvidia(api, ret));
        }

        Ok(temp)
    }
}

impl std::fmt::Display for NvidiaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl Error for NvidiaError {}
