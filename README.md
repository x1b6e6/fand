# fanD

Daemon for fan controlling like [fancontrol](https://github.com/lm-sensors/lm-sensors)

Allows using multiple temperature sources for controlling one fan. It's useful for water cooling systems when cpu and gpu connected in one path

Allows using complex algorithms for compute fan speed (by using JavaScript)

## Configuration

example:

```toml
[main]
# update interval in seconds
# 2 seconds by default
interval = 5

# register temperature source `myCpu` for getting cpu temperature
[source.myCpu]
# `file`: just read value from file and divide result by 1000 (standard hwmon format). field is required.
type = "file"

# by default in `*/hwmon*/temp*_input` values written in 'millicelcius' and must be multiple to 0.001 for getting celsius.
# 0.001 by default
factor = 0.001

# path to file for reading. required for `file` type
path = "/sys/devices/platform/nct6775.656/hwmon/hwmon1/temp13_input"

# register temperature source `myGpu` for getting gpu temperature
[source.myGpu]
# `nvidia`: using `libnvidia-ml.so` for getting data from proprietary driver
type = "nvidia"

# optional filter by device name
name = "NVIDIA GeForce RTX 4090"

# optional filter by device uuid
uuid = "GPU-23eda959-34a7-4abf-8e19-9c0beded366e"

# add fan
[[fan]]
# `pwm`: just write string with result in [0..255] to file
type = "pwm"

# path to pwm file. required for `pwm`
path = "/sys/devices/platform/nct6775.656/hwmon/hwmon2/pwm2"

# field for computing power of fan uses JavaScript engine
# Should produce float value from 0.0 to 1.0 where 0 is power off and 1 is full speed
value = "Math.max(myCpu, myGpu) / 100"
```
