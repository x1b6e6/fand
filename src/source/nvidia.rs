use super::{Source, Temperature};
use dlopen::wrapper::{Container, WrapperApi};
use std::{error::Error, ffi::CStr, fmt, num::NonZeroI32, ptr};

#[derive(Clone, Copy)]
#[repr(C)]
struct NvidiaDeviceHandle(ptr::NonNull<()>);

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
    device_handle_by_index:
        fn(index: u32, dev: &mut Option<NvidiaDeviceHandle>) -> Result<(), NvidiaError>,

    #[dlopen_name = "nvmlDeviceGetUUID"]
    device_get_uuid:
        fn(dev: NvidiaDeviceHandle, buf: *mut u8, size: u32) -> Result<(), NvidiaError>,

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

fn nvidia_init(nvidia: &'static mut Option<Nvidia>) -> &'static Nvidia {
    assert!(nvidia.is_none());

    let api: Container<NvidiaApi> =
        unsafe { Container::load("libnvidia-ml.so") }.expect("loading libnvidia-ml.so");

    api.init().expect("init nvidia backend");

    *nvidia = Some(Nvidia {
        api,
        devices: Vec::new(),
    });

    let nvidia = unsafe { nvidia.as_mut().unwrap_unchecked() };

    let mut cnt = 0;
    nvidia
        .api
        .devices_count(&mut cnt)
        .expect("get nvidia device count");

    for index in 0..cnt {
        let mut handle = None;
        nvidia
            .api
            .device_handle_by_index(index, &mut handle)
            .expect("create device handle");

        let handle = handle.unwrap();

        log::info!("Found {handle}");

        nvidia.devices.push(handle);
    }

    nvidia
}

fn nvidia() -> &'static Nvidia {
    static mut NVIDIA: Option<Nvidia> = None;

    #[allow(static_mut_refs)]
    match unsafe { NVIDIA.as_ref() } {
        None => nvidia_init(unsafe { &mut *&raw mut NVIDIA }),
        Some(nvidia) => nvidia,
    }
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
        uuid: Option<String>,
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
            log::error!("cannot deinit nvidia api: {err:?}");
        }
    }
}

impl SourceNvidia {
    pub fn new(name: Option<String>, uuid: Option<String>) -> Result<Self, SourceNvidiaError> {
        let nvidia = nvidia();

        let dev = match (&name, &uuid) {
            (Some(name), Some(uuid)) => nvidia
                .try_find_device(|dev| {
                    Ok::<_, NvidiaError>(
                        dev.try_get_name()? == name.as_str()
                            && dev.try_get_uuid()? == uuid.as_str(),
                    )
                })?
                .ok_or_else(|| SourceNvidiaError::NotFound {
                    name: Some(name.clone()),
                    uuid: Some(uuid.clone()),
                }),
            (None, Some(uuid)) => nvidia
                .try_find_device(|dev| Ok::<_, NvidiaError>(dev.try_get_uuid()? == uuid.as_str()))?
                .ok_or_else(|| SourceNvidiaError::NotFound {
                    name: None,
                    uuid: Some(uuid.clone()),
                }),
            (Some(name), None) => nvidia
                .try_find_device(|dev| Ok::<_, NvidiaError>(dev.try_get_name()? == name.as_str()))?
                .ok_or_else(|| SourceNvidiaError::NotFound {
                    name: Some(name.clone()),
                    uuid: None,
                }),
            (None, None) => nvidia.devices.first().ok_or(SourceNvidiaError::NoDevices),
        };

        let dev = *dev?;

        log::info!("Using {dev}");

        Ok(Self { dev })
    }
}

impl Source for SourceNvidia {
    fn try_get_temperature(&self) -> Result<Temperature, Box<dyn Error>> {
        let temp = self.dev.try_get_temperature()?;
        Ok(Temperature::from_celsius(temp as f32))
    }
}

impl NvidiaDeviceHandle {
    fn try_get_name(&self) -> Result<String, NvidiaError> {
        let mut buf = [0u8; 4096];
        nvidia()
            .api
            .device_get_name(*self, buf.as_mut_ptr(), buf.len() as u32)?;

        let name = unsafe { CStr::from_ptr(buf.as_ptr().cast()) };
        let name = unsafe { std::str::from_utf8_unchecked(name.to_bytes()) };

        Ok(name.to_string())
    }

    fn try_get_uuid(&self) -> Result<String, NvidiaError> {
        let mut buf = [0u8; 41];
        nvidia()
            .api
            .device_get_uuid(*self, buf.as_mut_ptr(), buf.len() as u32)?;

        let uuid = unsafe { CStr::from_ptr(buf.as_ptr().cast()) };
        let uuid = unsafe { std::str::from_utf8_unchecked(uuid.to_bytes()) };

        Ok(uuid.to_string())
    }

    fn try_get_temperature(&self) -> Result<u32, NvidiaError> {
        let mut temp = 0;
        let api = &nvidia().api;
        api.device_get_temperature(*self, 0, &mut temp)?;

        Ok(temp)
    }
}

impl fmt::Display for NvidiaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = nvidia().api.error_string(*self);
        let message = unsafe { CStr::from_ptr(message.cast()) };
        let message = unsafe { std::str::from_utf8_unchecked(message.to_bytes()) };

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

impl fmt::Display for NvidiaDeviceHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NvidiaDevice")
            .field(
                "name",
                &self
                    .try_get_name()
                    .unwrap_or_else(|_| "<ERROR>".to_string()),
            )
            .field(
                "uuid",
                &self
                    .try_get_uuid()
                    .unwrap_or_else(|_| "<ERROR>".to_string()),
            )
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::{NvidiaDeviceHandle, NvidiaError};
    use std::mem::size_of;

    #[test]
    fn nvidia_error_sizes() {
        assert_eq!(size_of::<NvidiaError>(), size_of::<i32>());
        assert_eq!(size_of::<Option<NvidiaError>>(), size_of::<NvidiaError>());
        assert_eq!(
            size_of::<Result<(), NvidiaError>>(),
            size_of::<NvidiaError>()
        );
    }

    #[test]
    fn nvidia_device_handle_sizes() {
        assert_eq!(size_of::<NvidiaDeviceHandle>(), size_of::<*const ()>());
        assert_eq!(
            size_of::<Option<NvidiaDeviceHandle>>(),
            size_of::<NvidiaDeviceHandle>()
        );
    }
}
