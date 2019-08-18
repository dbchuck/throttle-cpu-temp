extern crate futures;
extern crate futures_timer;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate log;
extern crate num_cpus;
extern crate simplelog;

use std::env;
use std::process;
use std::{thread, time};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::sync::{Arc, Mutex};
use simplelog::{CombinedLogger, Config, LogLevelFilter, SimpleLogger};
use std::time::Duration;
use futures_timer::Delay;
use futures::prelude::*;

// Sleep interval between temperature checking.
const SLEEP_TIME_MILLI: u64 = 500;

// Interval between frequency increase operation
const INCR_TIME_MILLI: u64 = 1000;

// Interval between frequency decrease operation
const DECR_TIME_MILLI: u64 = 100;

// File where minimum supported frequency should be collected.
const MIN_FREQ_FILE: &'static str = "/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_min_freq";

// File where maximum supported frequency should be collected.
const MAX_FREQ_FILE: &'static str = "/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq";

// Step size to change cpu frequency. 100Mhz step
const STEP_FREQ: u64 = 100000;

// Possible files where current temperature should be collected.
const POSSIBLE_TEMP_FILES: &'static [&'static str] = &[
	"/sys/class/thermal/thermal_zone1/temp",
	"/sys/class/thermal/thermal_zone2/temp",
	"/sys/class/hwmon/hwmon1/temp1_input",
	"/sys/class/hwmon/hwmon2/temp1_input",
	"/sys/class/hwmon/hwmon1/device/temp1_input",
	"/sys/class/hwmon/hwmon2/device/temp1_input",
];

// For spikes in temperature (a very sudden workload)
const DEACCR_RATIO: f64 = (1.0 / 4.0);

lazy_static! {
    static ref FREQUENCY: std::sync::Arc<std::sync::Mutex<u64>> =
        Arc::new(Mutex::new(max_frequency()));
}

fn parse_int_file(path: String) -> u64 {
    let mut content = String::new();
    let mut fp = File::open(path).unwrap();
    fp.read_to_string(&mut content).unwrap();
    content.trim().parse::<u64>().unwrap()
}

fn min_frequency() -> u64 {
    parse_int_file(MIN_FREQ_FILE.to_string())
}

fn max_frequency() -> u64 {
    parse_int_file(MAX_FREQ_FILE.to_string())
}

fn get_temp() -> u64 {
    // Gets the highest sensor temperature
    let mut max_temp = 0;
    for file in POSSIBLE_TEMP_FILES {
        if Path::new(file).exists() {
            let sensor_temp = parse_int_file(file.to_string()) / 1000;
            if max_temp < sensor_temp {
                max_temp = sensor_temp;
                info!("got temp from: {}", file);
            }
        }
    }
    if max_temp == 0 {
        error!("impossible to collect current cpu temperature!");
        process::exit(1);
    }
    return max_temp;
}

fn set_freq(freq: u64) {
    info!("setting frequency to {}", freq);
    for c in 0..num_cpus::get() {
        let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_max_freq", c);
        let mut fp = File::create(path).unwrap();
        fp.write_all(format!("{}\n", freq).as_bytes()).unwrap();
    }
}

fn main() {
    CombinedLogger::init(vec![
        SimpleLogger::new(LogLevelFilter::Info, Config::default()),
    ]).unwrap();

    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        error!("usage: {} <max temp>", args[0]);
        process::exit(1);
    }

    let max_temp: u64;
    match args[1].parse::<u64>() {
        Err(_) => {
            error!("invalid temperature: {}", args[1]);
            process::exit(1);
        }
        Ok(x) => max_temp = x,
    }
    info!("maximum temperature: {}", max_temp);
    info!("cpu count: {}", num_cpus::get());
    let min_freq: u64 = min_frequency();
    info!("minimum frequency supported: {}", min_freq);
    let max_freq: u64 = max_frequency();
    info!("maximum frequency supported: {}", max_freq);
    set_freq(*FREQUENCY.lock().unwrap());

    loop {
        let temp = get_temp();
        if temp > max_temp && *FREQUENCY.lock().unwrap() > min_freq {
            // decrease frequency
            let min_freq = min_freq.clone();
            thread::spawn(move || {
                let mut lock = FREQUENCY.try_lock();
                Delay::new(Duration::from_millis(DECR_TIME_MILLI))
                    .map(|()| {
                        if let Ok(ref mut cur_freq) = lock {
                            **cur_freq -= STEP_FREQ;
                            let temp_diff = temp - max_temp;
                            if temp_diff > 0 {
                                let new_freq =
                                    STEP_FREQ * ((DEACCR_RATIO * temp_diff as f64) as u64);
                                // need to check if the new frequency number wraps around max integer limit
                                if (**cur_freq - new_freq) > max_freq {
                                    **cur_freq = min_freq;
                                } else {
                                    **cur_freq -= new_freq;
                                }
                            }
                            if **cur_freq < min_freq {
                                **cur_freq = min_freq;
                            }
                            set_freq(cur_freq.clone());
                        }
                    })
                    .wait()
                    .unwrap();
            });
        } else if temp < (max_temp - 5) && *FREQUENCY.lock().unwrap() < max_freq {
            // increase frequency
            let max_freq = max_freq.clone();
            thread::spawn(move || {
                let mut lock = FREQUENCY.try_lock();
                Delay::new(Duration::from_millis(INCR_TIME_MILLI))
                    .map(|()| {
                        if let Ok(ref mut cur_freq) = lock {
                            **cur_freq += STEP_FREQ;
                            if **cur_freq > max_freq {
                                **cur_freq = max_freq;
                            }
                            set_freq(cur_freq.clone());
                        }
                    })
                    .wait()
                    .unwrap();
            });
            // .join()
            //     .expect("thread::spawn failed");
        }

        info!("current temperature: {}", temp);
        thread::sleep(time::Duration::from_millis(SLEEP_TIME_MILLI));
    }
}
