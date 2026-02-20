use crate::{
    fan::FanPower,
    source::{Source, Temperature},
};
use deno_core::{
    error::AnyError as DenoError, v8, Extension, FastString, JsRuntime, RuntimeOptions,
};
use std::{cell::RefCell, collections::HashMap, error::Error, rc::Rc};

pub struct Computed<'a> {
    formula: String,
    engine: &'a ComputeEngine,
}

enum CachedResult<T, E> {
    Some(T),
    Cached(T),
    Err(E),
}

struct EngineStaticValues {
    sources: HashMap<String, Rc<dyn Source>>,
    cache: HashMap<String, Temperature>,
}

pub struct ComputeEngine {
    js: RefCell<JsRuntime>,
}

static mut ENGINE_STATIC_VALUES: Option<EngineStaticValues> = None;

impl ComputeEngine {
    pub fn new(sources: HashMap<String, Rc<dyn Source>>) -> Self {
        #[allow(static_mut_refs)]
        unsafe {
            assert!(ENGINE_STATIC_VALUES.is_none())
        };

        unsafe {
            ENGINE_STATIC_VALUES = Some(EngineStaticValues {
                sources,
                cache: HashMap::new(),
            })
        };

        let js = JsRuntime::new(RuntimeOptions {
            extensions: vec![Extension {
                global_object_middleware: Some(Self::middleware),
                ..Default::default()
            }],
            ..Default::default()
        });

        Self {
            js: RefCell::new(js),
        }
    }

    fn static_values() -> &'static mut EngineStaticValues {
        #[allow(static_mut_refs)]
        unsafe {
            ENGINE_STATIC_VALUES.as_mut().unwrap()
        }
    }

    pub fn cache_invalidate(&self) {
        Self::static_values().cache.clear();
    }

    pub fn create_computed(&self, formula: &str) -> Computed<'_> {
        Computed {
            formula: String::from(formula),
            engine: self,
        }
    }

    fn value(name: &str) -> Option<CachedResult<Temperature, Box<dyn Error>>> {
        let cache = &mut Self::static_values().cache;
        let cached = cache.get(name);
        if let Some(&temperature) = cached {
            return Some(CachedResult::Cached(temperature));
        }

        let source = Self::static_values().sources.get(name);
        if let Some(source) = source {
            let result = source.try_get_temperature();
            match result {
                Ok(temperature) => {
                    cache.insert(name.to_string(), temperature);
                    Some(CachedResult::Some(temperature))
                }
                Err(err) => Some(CachedResult::Err(err)),
            }
        } else {
            None
        }
    }

    fn middleware<'s>(scope: &mut v8::HandleScope<'s>, value: v8::Local<'s, v8::Object>) {
        for (key, _) in &Self::static_values().sources {
            let name = v8::String::new(scope, key).unwrap();
            value.set_accessor(scope, name.into(), Self::accessor);
        }
    }

    fn accessor<'s>(
        scope: &mut v8::HandleScope<'s>,
        name: v8::Local<'s, v8::Name>,
        _: v8::PropertyCallbackArguments<'s>,
        mut ret: v8::ReturnValue,
    ) {
        let name = name.to_rust_string_lossy(scope);
        log::trace!("accessing {name}");
        let value = Self::value(&name);
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
}

impl<'a> Computed<'a> {
    pub fn try_compute(&self) -> Result<FanPower, DenoError> {
        let mut js = self.engine.js.borrow_mut();
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
