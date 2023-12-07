use crate::{fan::FanPower, source::Source};
use deno_core::{v8, Extension, FastString, JsRuntime, RuntimeOptions};
use log::{debug, error, trace};
use std::{collections::HashMap, convert::Infallible, mem::MaybeUninit, rc::Rc};

pub struct Computed {
    formula: String,
}

struct Sources {
    inited: bool,
    js: MaybeUninit<JsRuntime>,
    sources: MaybeUninit<Rc<HashMap<String, Rc<dyn Source>>>>,
}

static mut INSTANCE: Sources = Sources::null();

impl Sources {
    pub const fn null() -> Self {
        Self {
            inited: false,
            js: MaybeUninit::uninit(),
            sources: MaybeUninit::uninit(),
        }
    }

    pub fn js_mut(&mut self) -> &mut JsRuntime {
        self.check();
        unsafe { self.js.assume_init_mut() }
    }

    pub fn is_null(&self) -> bool {
        !self.inited
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
        self.inited = true;
    }

    fn accessor<'s>(
        scope: &mut v8::HandleScope<'s>,
        name: v8::Local<'s, v8::Name>,
        _: v8::PropertyCallbackArguments<'s>,
        mut ret: v8::ReturnValue,
    ) {
        let name = name.to_rust_string_lossy(scope);
        let source = unsafe { INSTANCE.sources.assume_init_ref() }.get(&name);
        if let Some(source) = source {
            trace!("accessing {name}");
            match source.as_ref().value() {
                Ok(value) => {
                    let value = value.celcius();
                    debug!("{name}: {value}");
                    ret.set_double(value as f64);
                }
                Err(e) => {
                    error!("{name}: {:?}", e);
                    ret.set_undefined();
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

impl Computed {
    pub fn new(value: &str, map: Rc<HashMap<String, Rc<dyn Source>>>) -> Result<Self, Infallible> {
        if unsafe { INSTANCE.is_null() } {
            unsafe { INSTANCE.init(map) }
        }

        let formula = String::from(value);

        Ok(Self { formula })
    }

    pub fn value(&self) -> Result<FanPower, ()> {
        let js = unsafe { INSTANCE.js_mut() };
        let result = js
            .execute_script(
                "[computed.rs:runtime.js]",
                FastString::Owned(Box::from(self.formula.as_str())),
            )
            .unwrap();

        let mut scope = js.handle_scope();
        let result = result.into_raw();
        let result = unsafe { result.as_ref() }.to_number(&mut scope).unwrap();
        let result = result.integer_value(&mut scope).unwrap();

        debug!("result: {result}");

        Ok(FanPower::from(result as u8))
    }
}
