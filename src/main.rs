#[macro_use]
extern crate dlopen_derive;

use crate::{
    config::{Config, ConfigFanTarget, ConfigSourceValue},
    fan::{Fan, FanPwm},
    source::{Source, SourceFile, SourceNvidia},
};
use clap::Parser as _;
use computed::Computed;
use config::{ConfigFan, ConfigMain, ConfigNvidiaFilter};
use std::{cell::RefCell, collections::HashMap, env, path::PathBuf, rc::Rc, str::FromStr as _};

mod cli;
mod computed;
mod config;
mod fan;
mod source;

fn main() {
    if env::var("RUST_LOG").is_err() {
        env::set_var("RUST_LOG", "info")
    }
    env_logger::init();

    let app = cli::App::parse();
    let path = PathBuf::from_str(
        app.config
            .as_ref()
            .map(String::as_str)
            .unwrap_or("/etc/fand/config.toml"),
    )
    .unwrap();

    let Config {
        sources,
        fans,
        main: ConfigMain { interval },
    } = Config::read_file(path).unwrap();

    let sources: HashMap<String, Rc<dyn Source>> = sources
        .into_iter()
        .map(|(name, source)| {
            let source: Rc<dyn Source> = match source {
                ConfigSourceValue::File { path, factor } => {
                    Rc::new(SourceFile::new(path, factor).unwrap())
                }
                ConfigSourceValue::Nvidia {
                    filter: ConfigNvidiaFilter { name, uuid },
                } => Rc::new(SourceNvidia::new(name, uuid).unwrap()),
            };
            (name, source)
        })
        .collect();

    let sources = Rc::new(sources);

    let mut fans: Vec<_> = fans
        .into_iter()
        .map(|fan| {
            let ConfigFan { value, target } = fan;
            let target: Rc<RefCell<dyn Fan>> = match target {
                ConfigFanTarget::Pwm { path } => Rc::new(RefCell::new(FanPwm::new(path).unwrap())),
            };
            let value = Computed::new(&value, sources.clone()).unwrap();
            (value, target)
        })
        .collect();

    loop {
        for (comp, fan) in fans.iter_mut() {
            let power = comp.value().unwrap();
            fan.as_ref().borrow_mut().write(power).unwrap();
        }
        std::thread::sleep(interval);
        computed::cache_invalidate();
    }
}
