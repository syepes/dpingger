//RUST_LOG=debug
#![feature(phase, slicing_syntax)]

#[phase(plugin)]
extern crate regex_macros;
extern crate regex;
#[phase(plugin, link)]
extern crate time;
#[phase(plugin, link)]
extern crate log;

use std::time::Duration;
use std::io::Timer;
use std::io::BufferedReader;
use std::io::File;
use std::io::fs::PathExtensions;
use std::io::process::{Command,ProcessOutput};

use std::str;
use std::task::spawn;
use std::comm::{channel, Sender, Receiver};
use std::sync::{Arc, RWLock};

use std::collections::HashMap;

use log::{Logger,LogRecord,LogLevel,LogLocation, set_logger};
use std::io::{ LineBufferedWriter, stdio, stderr} ;
// Custom Logger
struct CustomLogger {
    handle: LineBufferedWriter<stdio::StdWriter>,
}
// Implements Logger trait for Custom Logger which support logging timestamp, file name and line number in addition to log level, module path and message.
impl Logger for CustomLogger {
    fn log(&mut self, record: &LogRecord) {
        match writeln!(&mut self.handle,
                       "{}:{}:{}:{}:{} {}",
                       time::strftime("%Y-%m-%d %H:%M:%S %Z", &time::now()).unwrap(),
                       record.level,
                       record.module_path,
                       "",
                       record.line,
                       record.args) {
            Err(e) => panic!("failed to log: {}", e),
            Ok(()) => {}
        }
    }
}

#[allow(unused_must_use)]
#[cfg(target_os="linux")]
fn ping(host: String, interval: int, sender: Sender<HashMap<String, String>>, ctrl: Arc<RWLock<int>>) {
    log::set_logger(box CustomLogger { handle: stderr() } );
    let mut timer = Timer::new().unwrap();
    println!("ping(): Starting ({}sec) - {}", interval, host);

    loop {
        let mut data: HashMap<String, String> = HashMap::new();
        let mut cmd = Command::new("ping");
        cmd.args(&["-nqc", "2", "-w", "3", host.as_slice()]);
        debug!("ping(): cmd: {}", cmd);

        // Spawn a process, wait for it to finish, and collect it's output
        match cmd.output() {
            //Err(why) => panic!("Couldn't spawn cmd: {}", why.desc),
            Err(why) => panic!("ping(): Couldn't spawn cmd: {}", why),
            Ok(ProcessOutput { error: err, output: out, status: exit }) => {
                //time::now_utc()
                let timespec = time::get_time();
                let ts_ms = timespec.sec + timespec.nsec as i64 / 1000 / 1000;

                // Check if the process succeeded, i.e. the exit code was 0
                if exit.success() {
                    let so: &str = str::from_utf8(out.as_slice()).unwrap();
                    let re = regex!(r"(?is).*\s([\d\.,]+)% packet loss.*= ([\d\.,]+)/([\d\.,]+)/([\d\.,]+)/([\d\.,]+) ms.*");

                    if re.is_match(so) {
                        for cap in re.captures_iter(so) {
                            data.insert("host".to_string(), host.to_string());
                            data.insert("ts".to_string(), ts_ms.to_string());
                            data.insert("loss_pct".to_string(), cap.at(1).to_string());
                            data.insert("max".to_string(), cap.at(2).to_string());
                            data.insert("min".to_string(), cap.at(3).to_string());
                            data.insert("avg".to_string(), cap.at(4).to_string());

                        }
                    } else {
                        error!("ping(): Could not extract ping metrics");
                    }
                    debug!("ping(): cmd.status: {}", cmd.status());

                } else {
                    let so: &str = str::from_utf8(out.as_slice()).unwrap();
                    let se: &str = str::from_utf8(err.as_slice()).unwrap();
                    let re = regex!(r"(?is).*\s([\d\.,]+)% packet loss.*");

                    if re.is_match(so) {
                        for cap in re.captures_iter(so) {
                            data.insert("host".to_string(), host.to_string());
                            data.insert("ts".to_string(), ts_ms.to_string());
                            data.insert("loss_pct".to_string(), cap.at(1).to_string());
                        }
                        warn!("ping(): Failed");
                    } else {
                        error!("ping(): unknown error: {}", cmd.status());
                        debug!("ping(): stdout was:\n{}", so);
                        debug!("ping(): stderr was:\n{}", se);
                    }
                }
            },
        }

        debug!("ping(): data = {}", data);
        if !data.is_empty() {
            sender.send_opt(data);
        }

        let ctrl_msg = ctrl.read();
        if ctrl_msg.to_string() != "0" {
            println!("ping(): Stopping due to signal from workers() ({})", ctrl_msg.to_string());
            break;
        }

        timer.sleep(Duration::seconds(interval as i64));
    }
    println!("ping(): Done");
}

#[allow(unused_must_use)]
#[cfg(not(target_os = "linux"))]
fn ping(host: String, interval: int, sender: Sender<HashMap<String, String>>, ctrl: Arc<RWLock<int>>) {
    log::set_logger(box CustomLogger { handle: stderr() } );
    let mut timer = Timer::new().unwrap();
    println!("ping(): Starting ({}sec) - {}", interval, host);

    loop {
        let mut data: HashMap<String, String> = HashMap::new();
        let mut cmd = Command::new("ping");
        cmd.args(&["-n", "2", "-w", "3", host.as_slice()]);
        debug!("ping(): cmd: {}", cmd);

        // Spawn a process, wait for it to finish, and collect it's output
        match cmd.output() {
            //Err(why) => panic!("Couldn't spawn cmd: {}", why.desc),
            Err(why) => panic!("ping(): Couldn't spawn cmd: {}", why),
            Ok(ProcessOutput { error: err, output: out, status: exit }) => {
                //time::now_utc()
                let timespec = time::get_time();
                let ts_ms = timespec.sec + timespec.nsec as i64 / 1000 / 1000;

                // Check if the process succeeded, i.e. the exit code was 0
                if exit.success() {
                    let so: &str = str::from_utf8(out.as_slice()).unwrap();
                    let re = regex!(r"(?is).*\s\(([\d\.,]+)% loss\).*Minimum = (\d+)ms.*Maximum = (\d+)ms.*Average = (\d+)ms.*");

                    if re.is_match(so) {
                        for cap in re.captures_iter(so) {
                            data.insert("host".to_string(), host.to_string());
                            data.insert("ts".to_string(), ts_ms.to_string());
                            data.insert("loss_pct".to_string(), cap.at(1).to_string());
                            data.insert("min".to_string(), cap.at(2).to_string());
                            data.insert("max".to_string(), cap.at(3).to_string());
                            data.insert("avg".to_string(), cap.at(4).to_string());
                        }
                    } else {
                        error!("ping(): Could not extract ping metrics");
                    }
                    debug!("ping(): cmd.status: {}", cmd.status());

                } else {
                    let so: &str = str::from_utf8(out.as_slice()).unwrap();
                    let se: &str = str::from_utf8(err.as_slice()).unwrap();
                    let re = regex!(r"(?is).*\s\(([\d\.,]+)% loss\).*");

                    if re.is_match(so) {
                        for cap in re.captures_iter(so) {
                            data.insert("host".to_string(), host.to_string());
                            data.insert("ts".to_string(), ts_ms.to_string());
                            data.insert("loss_pct".to_string(), cap.at(1).to_string());
                        }
                        warn!("ping(): Failed");
                    } else {
                        error!("ping(): unknown error: {}", cmd.status());
                        debug!("ping(): stdout was:\n{}", so);
                        debug!("ping(): stderr was:\n{}", se);
                    }
                }
            },
        }

        debug!("ping(): data = {}", data);
        if !data.is_empty() {
            sender.send_opt(data);
        }

        let ctrl_msg = ctrl.read();
        if ctrl_msg.to_string() != "0" {
            println!("ping(): Stopping due to signal from workers() ({})", ctrl_msg.to_string());
            break;
        }

        timer.sleep(Duration::seconds(interval as i64));
    }
    println!("ping(): Done");
}


#[allow(unused_must_use)]
fn workers(hosts: &[String], receive_from_main:  Receiver<int>, send_to_main: Sender<int>) {
    println!("workers(): Starting - {}", hosts);

    let mut rx_metrics_cnt: int = 0;
    let mut timer = Timer::new().unwrap();
    let ctrl: Arc<RWLock<int>> = Arc::new(RWLock::new(0i));
    let (sender_to_ping, receive_from_ping): (Sender<HashMap<String, String>>, Receiver<HashMap<String, String>>) = channel();

    for h in hosts.iter() {
        let sender_to_ping_task = sender_to_ping.clone();
        let ctrl_task = ctrl.clone();
        let h_task = h.clone();

        spawn(move|| {
            ping(h_task, 5, sender_to_ping_task, ctrl_task);
        });
    }

    loop {
        if receive_from_main.try_recv().is_ok() {
            println!("workers(): Stopping due to signal from main()");

            // Send stop to all pings
            println!("workers(): Sending abort to ping()");
            let mut ctrl_w = ctrl.write();
            *ctrl_w = 1;

            // Send back to the main the # of received metrics
            send_to_main.send_opt(rx_metrics_cnt);
            break;
        }

        let ping_result = receive_from_ping.try_recv();
        if ping_result.is_ok() {
            println!("workers(): ping_result = {}", ping_result.unwrap());
            rx_metrics_cnt += 1
        }

        timer.sleep(Duration::milliseconds(30));
    }
    send_to_main.send_opt(rx_metrics_cnt);
    println!("workers(): Done");
}

// Workaround untill SIGKILL can we handheld by Rust in Unix and Windows
fn stop_action() -> bool {
    Path::new("stop.txt").exists()
}

#[allow(unused_must_use)]
fn main() {
    log::set_logger(box CustomLogger { handle: stderr() } );

    // Read list of hosts
    let path = Path::new("hosts.txt");
    let mut file = BufferedReader::new(File::open(&path));
    let mut hosts: Vec<String> = file.read_to_string().unwrap().lines().map(|l| l.trim().to_string()).collect();
    //let mut hosts: Vec<String> = vec!("dns.google.com".to_string(),"localhost".to_string());

    println!("hosts (File): {}", hosts);
    println!("main(): Start");
    let mut stop_action_sent: bool = false;
    let mut timer = Timer::new().unwrap();
    let (send_from_worker_to_main, receive_from_worker): (Sender<int>, Receiver<int>) = channel();
    let (send_from_main_to_worker, receive_from_main): (Sender<int>, Receiver<int>) = channel();

    spawn(move|| {
        workers(hosts[], receive_from_main, send_from_worker_to_main);
    });

    loop {
        let data = receive_from_worker.try_recv();
        if data.is_ok() {
            println!("main(): data = {}", data);
            break;
        }
        if stop_action() && !stop_action_sent {
            println!("main(): Sending abort to worker()");
            send_from_main_to_worker.send_opt(0);
            stop_action_sent = true;
        }
        timer.sleep(Duration::seconds(1));
    }

    println!("main(): Done");
    std::os::set_exit_status(0);
}
