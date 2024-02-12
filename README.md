# fanD

Daemon for fan controlling like [fancontrol](https://github.com/lm-sensors/lm-sensors)

Allows using multiple temperature sources for controlling one fan. It's useful for water cooling systems when cpu and gpu connected in one path

Allows using complex algorithms for compute fan speed (by using JavaScript)

## Build and run

```shell
cargo build --release
sudo ./target/release/fand
```

## Usage

```
Usage: fand [OPTIONS]

Options:
  -c, --config <PATH>  [default: /etc/fand/config.toml]
  -h, --help           Print help
```

## Configuration

Configuration read from `/etc/fand/config.toml` by default

For working needs at least one `source.XXX` section and at least one `fan` value

---

### `main` section

Base properties:

- `interval` update interval in seconds (`2` by default)

_example:_

```toml
[main]
interval = 5
```

---

### source `file`

Reading temperature from file

Properties:

- `path` path to file for reading. required for `file` type
- `factor` multiplier for values from file (`0.001` by default)

_example:_

```toml
[source.myCpu]
type = "file"
path = "/sys/devices/platform/nct6775.656/hwmon/hwmon1/temp13_input"

# values in `nct6775` driver written in 'millicelcius' and must be multiplied by 0.001
factor = 0.001
```

---

### source `nvidia`

Get temperature from nvidia devices. `libnvidia-ml.so` must be exists in the system

Properties:

- `name` select card by name. optional
- `uuid` select card by uuid. optional

You can found `name` and `uuid` for all your cards at starting `fand` with correctly configured `nvidia` source section

_log example:_

```log
[2024-02-11T15:05:18Z INFO  fand::source::nvidia] Found NvidiaDevice { name: "NVIDIA GeForce RTX 5000", uuid: "GPU-23eda959-34a7-4abf-8e19-9c0beded366e" }
```

_example:_

```toml
[source.myGpu]
type = "nvidia"
name = "NVIDIA GeForce RTX 5000"
uuid = "GPU-23eda959-34a7-4abf-8e19-9c0beded366e"
```

---

### fan `pwm`

Write fan power to file in text format (values in range `0..=255`)

Properties:

- `path` path to pwm file. required for `pwm` type
- `value` js code for computing result. required for `pwm` type

`value` must return double in range `0.0..=1.0` where `0.0` is power off and `1.0` is full speed

_example:_

```toml
[[fan]]
type = "pwm"
path = "/sys/devices/platform/nct6775.656/hwmon/hwmon2/pwm2"
value = '''
    var calc;
    if (!calc) calc = (minTemp, temp, maxTemp) => Math.min(1, (Math.max(temp, minTemp) - minTemp) / (maxTemp - minTemp) );
    Math.max(1, calc(30, myCpu, 80), calc(30, myGpu, 40)) // result of last line will be used as power
'''
```
