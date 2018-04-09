extern crate chrono;
#[macro_use]
extern crate log;
extern crate num_cpus;
extern crate simplelog;
extern crate timer;

use std::env;
use std::process;
use std::{thread, time};
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use simplelog::{CombinedLogger, Config, LogLevelFilter, SimpleLogger};

// Sleep interval between temperature checking.
const SLEEP_TIME_MILLI: u64 = 200;


// File where minimum supported frequency should be colected.
const MIN_FREQ_FILE: &'static str = "/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_min_freq";

// File where maximum supported frequency should be colected.
const MAX_FREQ_FILE: &'static str = "/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq";

// Step size to change cpu frequency.
const STEP_FREQ: u64 = 100000;

// Possible files where current temperature should be collected.
const POSSIBLE_TEMP_FILES: &'static [&'static str] = &["/sys/class/hwmon/hwmon2/temp1_input"];

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
    for file in POSSIBLE_TEMP_FILES {
        if Path::new(file).exists() {
            return parse_int_file(file.to_string()) / 1000;
        }
    }
    error!("impossible to collect current cpu temperature!");
    process::exit(1);
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

    let min_freq = min_frequency();
    info!("minimum frequency supported: {}", min_freq);
    let max_freq = max_frequency();
    info!("maximum frequency supported: {}", max_freq);

    let mut cur_freq = max_freq;
    set_freq(cur_freq);
    let timer = timer::Timer::new();
    let tick = chan::tick_ms(100);
    let boom = chan::after_ms(500);
    loop {
        chan_select! {
            default => {
                println!("    .");
                thread::sleep(std::time::Duration::from_millis(50));
            },
            tick.recv() => println!("tick."),
            boom.recv() => { println!("BOOM!"); return; },
        }
    }
    loop {
        let temp = get_temp();
        if temp > max_temp && cur_freq > min_freq {
            cur_freq -= STEP_FREQ;
            if cur_freq < min_freq {
                cur_freq = min_freq;
            }
            set_freq(cur_freq);
        } else if temp < (max_temp - 5) && cur_freq < max_freq {
            cur_freq += STEP_FREQ;
            if cur_freq > max_freq {
                cur_freq = max_freq;
            }
            set_freq(cur_freq);
            // timer.schedule_with_delay(chrono::Duration::seconds(1), move || {
            //
            // });
        }
        debug!("current temperature: {}", temp);
        thread::sleep(time::Duration::from_millis(SLEEP_TIME_MILLI));
    }
}
