use serde::Deserialize;
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Milliseconds(u64);

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "type")]
pub enum ConfigSourceValue {
    #[serde(rename = "file")]
    File { path: PathBuf },
    #[serde(rename = "nvidia")]
    Nvidia {
        name: Option<String>,
        index: Option<u32>,
    },
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "type")]
pub enum ConfigFanTarget {
    #[serde(rename = "pwm")]
    Pwm { path: PathBuf },
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct ConfigFan {
    pub value: String,
    #[serde(flatten)]
    pub target: ConfigFanTarget,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct ConfigMain {
    #[serde(default)]
    pub interval: Duration,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
pub struct Config {
    pub main: ConfigMain,
    #[serde(rename = "source")]
    pub sources: HashMap<String, ConfigSourceValue>,
    #[serde(rename = "fan")]
    pub fans: Vec<ConfigFan>,
}

#[derive(Debug)]
pub enum ConfigReadError {
    Io(io::Error),
    Toml(toml::de::Error),
}

impl Config {
    pub fn read_file<P>(path: P) -> Result<Self, ConfigReadError>
    where
        P: AsRef<Path>,
    {
        let root = fs::read_to_string(path)?;
        let config: Self = toml::from_str(&root)?;

        Ok(config)
    }
}

impl From<io::Error> for ConfigReadError {
    fn from(value: io::Error) -> Self {
        ConfigReadError::Io(value)
    }
}

impl From<toml::de::Error> for ConfigReadError {
    fn from(value: toml::de::Error) -> Self {
        ConfigReadError::Toml(value)
    }
}

impl Default for Milliseconds {
    fn default() -> Self {
        Milliseconds(5000)
    }
}

#[cfg(test)]
mod test {
    use std::{path::PathBuf, time::Duration};

    use crate::config::{Config, ConfigFanTarget, ConfigSourceValue};

    #[test]
    fn parse() {
        const CONF: &str = r#"
[main]
interval = { secs = 123, nanos = 0 }

[source.s1]
type = "file"
path = "/value"

[source.s2]
type = "nvidia"

[[fan]]
type = "pwm"
value = "s3"
path = "/pwm"
"#;
        let config: Config = toml::from_str(CONF).unwrap();

        assert_eq!(config.sources.len(), 2);
        assert_eq!(config.fans.len(), 1);

        assert_eq!(config.main.interval, Duration::from_secs(123));

        assert!(config.sources.contains_key("s1"));
        assert_eq!(
            config.sources["s1"],
            ConfigSourceValue::File {
                path: PathBuf::from("/value")
            }
        );

        assert!(config.sources.contains_key("s2"));
        assert_eq!(
            config.sources["s2"],
            ConfigSourceValue::Nvidia {
                name: None,
                index: None
            }
        );

        assert_eq!(config.fans[0].value, "s3");
        assert_eq!(
            config.fans[0].target,
            ConfigFanTarget::Pwm {
                path: PathBuf::from("/pwm")
            }
        );
    }
}
