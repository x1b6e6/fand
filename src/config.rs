use serde::{Deserialize, Deserializer};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, PartialEq, Deserialize)]
pub struct Milliseconds(u64);

#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum ConfigSourceValue {
    #[serde(rename = "file")]
    File { path: PathBuf, factor: Option<f32> },
    #[serde(rename = "nvidia")]
    Nvidia {
        name: Option<String>,
        uuid: Option<String>,
    },
}

#[derive(Debug, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum ConfigFanTarget {
    #[serde(rename = "pwm")]
    Pwm { path: PathBuf },
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct ConfigFan {
    pub value: String,
    #[serde(flatten)]
    pub target: ConfigFanTarget,
}

#[derive(Debug, PartialEq, Deserialize)]
pub struct ConfigMain {
    #[serde(default = "ConfigMain::interval_default")]
    #[serde(deserialize_with = "ConfigMain::interval_deserialize")]
    pub interval: Duration,
}

impl ConfigMain {
    fn interval_default() -> Duration {
        Duration::from_secs(2)
    }

    fn interval_deserialize<'de, D>(d: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Deserialize::deserialize(d)?;
        Ok(Duration::from_secs(value))
    }
}

#[derive(Debug, PartialEq, Deserialize)]
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
interval = 1

[source.s1]
type = "file"
path = "/value"

[source.s2]
type = "nvidia"

[source.s3]
type = "nvidia"
name = "NVIDIA GeForce RTX 4090"

[source.s4]
type = "nvidia"
uuid = "GPU-23eda959-34a7-4abf-8e19-9c0beded366e"

[source.s5]
type = "file"
factor = 0.1
path = "/value2"

[[fan]]
type = "pwm"
value = "s3"
path = "/pwm"
"#;
        let config: Config = toml::from_str(CONF).unwrap();

        assert_eq!(config.sources.len(), 5);
        assert_eq!(config.fans.len(), 1);

        assert_eq!(config.main.interval, Duration::from_secs(123));

        assert!(config.sources.contains_key("s1"));
        assert_eq!(
            config.sources["s1"],
            ConfigSourceValue::File {
                path: PathBuf::from("/value"),
                factor: None,
            }
        );

        assert!(config.sources.contains_key("s2"));
        assert_eq!(
            config.sources["s2"],
            ConfigSourceValue::Nvidia {
                name: None,
                uuid: None
            }
        );

        assert!(config.sources.contains_key("s3"));
        assert_eq!(
            config.sources["s3"],
            ConfigSourceValue::Nvidia {
                name: Some("NVIDIA GeForce RTX 4090".to_string()),
                uuid: None
            }
        );

        assert!(config.sources.contains_key("s4"));
        assert_eq!(
            config.sources["s4"],
            ConfigSourceValue::Nvidia {
                name: None,
                uuid: Some("GPU-23eda959-34a7-4abf-8e19-9c0beded366e".to_string()),
            }
        );

        assert!(config.sources.contains_key("s5"));
        assert_eq!(
            config.sources["s5"],
            ConfigSourceValue::File {
                path: PathBuf::from("/value2"),
                factor: Some(0.1)
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
