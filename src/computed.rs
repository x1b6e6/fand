use crate::{
    fan::FanPower,
    source::{Source, Temperature},
};
use deno_core::{
    error::AnyError as DenoError, v8, Extension, FastString, JsRuntime, RuntimeOptions,
};
use std::{cell::RefCell, collections::HashMap, error::Error, mem::MaybeUninit, rc::Rc};

pub struct Computed {
    formula: String,
}

struct Sources {
    inited: bool,
    js: MaybeUninit<JsRuntime>,
    sources: MaybeUninit<Rc<HashMap<String, Rc<dyn Source>>>>,
    cache: RefCell<MaybeUninit<HashMap<String, Temperature>>>,
}

enum CachedResult<T, E> {
    Some(T),
    Cached(T),
    Err(E),
}

static mut INSTANCE: Sources = Sources::null();

impl Sources {
    pub const fn null() -> Self {
        Self {
            inited: false,
            js: MaybeUninit::uninit(),
            sources: MaybeUninit::uninit(),
            cache: RefCell::new(MaybeUninit::uninit()),
        }
    }

    pub fn js_mut(&mut self) -> &mut JsRuntime {
        self.check();
        unsafe { self.js.assume_init_mut() }
    }

    pub fn is_null(&self) -> bool {
        !self.inited
    }

    pub fn cache_invalidate(&mut self) {
        unsafe { self.cache.borrow_mut().assume_init_mut() }.clear()
    }

    pub fn value(&self, name: &str) -> Option<CachedResult<Temperature, Box<dyn Error>>> {
        let cache = self.cache.borrow();
        let cached = unsafe { cache.assume_init_ref() }.get(name);
        if let Some(&temperature) = cached {
            return Some(CachedResult::Cached(temperature));
        }

        drop(cache);

        let source = unsafe { self.sources.assume_init_ref() }.get(name);
        if let Some(source) = source {
            let result = source.try_get_temperature();
            match result {
                Ok(temperature) => {
                    unsafe { self.cache.borrow_mut().assume_init_mut() }
                        .insert(name.to_string(), temperature);
                    Some(CachedResult::Some(temperature))
                }
                Err(err) => Some(CachedResult::Err(err)),
            }
        } else {
            None
        }
    }

    fn check(&self) {
        if self.is_null() {
            panic!("not initialized");
        }
    }

    fn init(&mut self, map: Rc<HashMap<String, Rc<dyn Source>>>) {
        self.sources.write(map);
        self.js.write(JsRuntime::new(RuntimeOptions {
            extensions: vec![Extension {
                global_object_middleware: Some(Self::middleware),
                ..Default::default()
            }],
            ..Default::default()
        }));
        self.cache.borrow_mut().write(HashMap::new());
        self.inited = true;
    }

    fn accessor<'s>(
        scope: &mut v8::HandleScope<'s>,
        name: v8::Local<'s, v8::Name>,
        _: v8::PropertyCallbackArguments<'s>,
        mut ret: v8::ReturnValue,
    ) {
        let name = name.to_rust_string_lossy(scope);
        log::trace!("accessing {name}");
        let value = unsafe { INSTANCE.value(&name) };
        if let Some(value) = value {
            match value {
                CachedResult::Cached(temperature) => {
                    log::debug!("using cached value for {name}: {temperature:8}");
                    ret.set_double(temperature.celsius() as f64);
                }
                CachedResult::Some(temperature) => {
                    log::debug!("{name}: {temperature:8}");
                    ret.set_double(temperature.celsius() as f64);
                }
                CachedResult::Err(err) => {
                    log::error!("cannot get temperature for {name}: {err:?}");
                    let exception = v8::String::new(
                        scope,
                        &format!("cannot get temperature for {name}: {err:?}"),
                    )
                    .unwrap();
                    scope.throw_exception(exception.into());
                }
            }
        }
    }

    fn middleware<'s>(scope: &mut v8::HandleScope<'s>, value: v8::Local<'s, v8::Object>) {
        for (key, _) in unsafe { INSTANCE.sources.assume_init_ref().as_ref() } {
            let name = v8::String::new(scope, key).unwrap();
            value.set_accessor(scope, name.into(), Self::accessor);
        }
    }
}

pub fn cache_invalidate() {
    unsafe { INSTANCE.cache_invalidate() };
}

impl Computed {
    pub fn new(value: &str, map: Rc<HashMap<String, Rc<dyn Source>>>) -> Self {
        if unsafe { INSTANCE.is_null() } {
            unsafe { INSTANCE.init(map) }
        }

        let formula = String::from(value);

        Self { formula }
    }

    pub fn try_compute(&self) -> Result<FanPower, DenoError> {
        let js = unsafe { INSTANCE.js_mut() };
        let result = js.execute_script(
            "[computed.rs:runtime.js]",
            FastString::Owned(Box::from(self.formula.as_str())),
        )?;

        let mut scope = js.handle_scope();
        let result = result.into_raw();
        let result = unsafe { result.as_ref() };
        let power = if !result.is_number() {
            log::warn!("computed value {result:?} is not a number. Set full speed");
            FanPower::full_speed()
        } else {
            let power = unsafe { result.to_number(&mut scope).unwrap_unchecked() };
            let power = power.value();
            let power = f64::min(f64::max(0.0, power), 1.0);
            let power = FanPower::from((power * 255.0) as u8);
            log::debug!("computed power: {power:7.2}");

            power
        };

        Ok(power)
    }
}
