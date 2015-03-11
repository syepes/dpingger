//RUST_LOG=debug
#![feature(slicing_syntax)]
#![feature(phase)]
#[phase(plugin, link)] extern crate log;
#[phase(plugin, link)] extern crate regex_macros;
#[phase(plugin, link)] extern crate regex;

extern crate time;

use std::io::BufferedReader;
use std::io::File;
use std::io::fs::PathExtensions;

use std::str;
use std::io::process::{Command,ProcessOutput};
use std::collections::HashMap;

use std::io::Timer;
use std::time::Duration;

use std::task::spawn;
use std::comm::{channel, Sender, Receiver};
use std::sync::{Arc, RWLock};



#[allow(unused_must_use)]
#[cfg(not(target_os = "linux"))]
fn ping<'a>(host: &'a String, interval: int, sender: Sender<HashMap<String, String>>, ctrl: Arc<RWLock<int>>) {
    let mut timer = Timer::new().unwrap();

    println!("ping(): Starting ({}sec)", interval);

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
#[cfg(target_os="linux")]
fn ping(host: String, interval: int, sender: Sender<HashMap<String, String>>, ctrl: Arc<RWLock<int>>) {
    let mut timer = Timer::new().unwrap();

    println!("ping(): Starting ({}sec)", interval);

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
//fn workers(hosts: &Vec<String>, receive_from_main: Receiver<int>, send_to_main: Sender<int>) {
//fn workers<'a>(hosts: &'a [String], receive_from_main: Receiver<int>, send_to_main: Sender<int>) {
fn workers<'a>(hosts: &'a Vec<String>, receive_from_main: Receiver<int>, send_to_main: Sender<int>) {
    let ctrl: Arc<RWLock<int>> = Arc::new(RWLock::new(0i));
    let (sender_to_ping, receive_from_ping) = channel();

    let mut rx_metrics_cnt: int = 0;
    let mut timer = Timer::new().unwrap();


    for h in hosts.mut_iter() {
        let sender_to_ping_task = sender_to_ping.clone();
        let ctrl_local = ctrl.clone();
        println!("hh: {}",h);
        spawn(move|| {
            ping(h, 5, sender_to_ping_task, ctrl_local);
        });

    }

    loop {
        if receive_from_main.try_recv().is_ok() {
            println!("workers(): Stopping due to signal from main()");

            // Send stop to all pings
            println!("workers(): Sending abort to ping()");
            let mut ctrl_w = ctrl.write();
            *ctrl_w = 1;

            send_to_main.send_opt(rx_metrics_cnt);
            break;
        }

        let ping_result = receive_from_ping.try_recv();
        if ping_result.is_ok() {
            println!("workers(): ping_result = {}", ping_result.unwrap());
            rx_metrics_cnt += 1
        } else {
            //println!("workers(): NOK ping_result = {}", ping_result);
        }
        timer.sleep(Duration::milliseconds(60));
    }
    send_to_main.send_opt(rx_metrics_cnt);
}

fn stop_action() -> bool {
    Path::new("stop.txt").exists()
}

#[allow(unused_must_use)]
fn main() {
    if log_enabled!(log::DEBUG) {
        println!("!debugging!");
    }

    //let hosts = ["localhost", "dns.google.com", "bsm", "om"];
    //let hosts = ["om","localhost"];
    //let hosts = ["localhost"];
    //let mut hosts: Vec<&str> = Vec::new();
    let mut hosts: Vec<String> = vec!("om".to_string(),"localhost".to_string());
    //let mut hosts: Vec<&str> = vec!("om");

    let path = Path::new("nodes.txt");
    let mut file = BufferedReader::new(File::open(&path));
    //
    //
    //
    //let hosts_file: Vec<&str> = file.lines().map(|x| x.as_slice() ).collect();
    //let hosts_file: Vec<String> = file.lines().map(|x| x.unwrap().as_ref() ).collect();
    //for i in h.iter() {
    //    println!("h: {}", i);
    //    let l:String = i.clone();
    //    hosts.push(l.as_slice());
    //}
    //
    //for line in file.lines(){
    //    let l:String = line.unwrap().clone();
    //    hosts.push(l.as_slice());
    //}


    println!("hosts: {}", hosts);
    //println!("hosts (File): {}", hosts_file);

    println!("main(): Start");
    let (send_from_worker_to_main, receive_from_worker) = channel();
    let (send_from_main_to_worker, receive_from_main) = channel();
    let mut timer = Timer::new().unwrap();

    spawn(move|| {
        workers(hosts[], receive_from_main, send_from_worker_to_main);
    });


    loop {
        let data = receive_from_worker.try_recv();
        if data.is_ok() {
            println!("main(): data = {}", data);
            break;
        } else {
            //println!("main(): NOK data = {}", data);
        }

        if stop_action() {
            println!("main(): Sending abort to worker()");
            send_from_main_to_worker.send_opt(0);
        }
        timer.sleep(Duration::seconds(1));
    }

    println!("main(): Done");
    std::os::set_exit_status(0);
}

