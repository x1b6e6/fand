use signal_hook::{consts::TERM_SIGNALS, low_level::register as signal_handler};

pub fn init() {
    unsafe {
        for &signal in TERM_SIGNALS {
            signal_handler(signal, move || {
                log::error!("Signal {signal} received. Panic!");
                panic!()
            })
            .unwrap();
        }
    }
}
