//RUST_LOG=debug
#![allow(unused_mut)]
#![feature(plugin,path_ext,scoped,duration)]

// https://github.com/ujh/iomrascalai/blob/ee1af121ec67a85701e5533f02f66e4cd37083ff/src/main.rs
// http://stackoverflow.com/questions/28490170/entry-point-could-not-be-located-when-running-program-on-windows
#![plugin(regex_macros)] #[no_link]
extern crate regex_macros;
extern crate regex;

#[macro_use] extern crate log;
extern crate time;

use std::time::Duration;
use std::process::{Command,Output};

use std::path::Path;
use std::fs::metadata;
use std::fs::File;

//http://stackoverflow.com/questions/29216271/creating-a-vector-of-strings-using-the-new-stdfsfile
//https://github.com/phildawes/racer/blob/master/src/racer/util.rs#L12
use std::io::{BufRead, BufReader};

use std::process;
use std::str;
use std::thread;
//use std::thread::{Thread,JoinGuard};
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::{Arc, RwLock};

use std::net;

use std::collections::HashMap;
use std::result::Result;



#[allow(unused_must_use)]
#[cfg(target_os="linux")]
fn ping(host: String, interval: isize, sender: Sender<HashMap<String, String>>, ctrl: Arc<RwLock<isize>>) {
    println!("ping():{}: Starting ({}sec) - {}", thread::current().name().unwrap(), interval, host);

    loop {
        let mut data: HashMap<String, String> = HashMap::new();
        let mut cmd = Command::new("ping");
        cmd.args(&["-nqc", "2", "-w", "3", &host]);
        debug!("ping(): cmd: {}", cmd);

        // Spawn a process, wait for it to finish, and collect it's output
        match cmd.output() {
            //Err(why) => panic!("Couldn't spawn cmd: {}", why.desc),
            Err(why) => panic!("ping(): Couldn't spawn cmd: {}", why),
            Ok(Output { stderr: err, stdout: out, status: exit }) => {
                //time::now_utc()
                let timespec = time::get_time();
                let ts_ms = timespec.sec + timespec.nsec as i64 / 1000 / 1000;

                // Check if the process succeeded, i.e. the exit code was 0
                if exit.success() {
                    let so: &str = str::from_utf8(&out).unwrap();
                    let re = regex!(r"(?is).*\s([\d\.,]+)% packet loss.*= ([\d\.,]+)/([\d\.,]+)/([\d\.,]+)/([\d\.,]+) ms.*");

                    if re.is_match(so) {
                        for cap in re.captures_iter(so) {
                            data.insert("host".to_string(), host.to_string());
                            data.insert("ts".to_string(), ts_ms.to_string());
                            data.insert("loss_pct".to_string(), cap.at(1).unwrap().to_string());
                            data.insert("max".to_string(), cap.at(2).unwrap().to_string());
                            data.insert("min".to_string(), cap.at(3).unwrap().to_string());
                            data.insert("avg".to_string(), cap.at(4).unwrap().to_string());

                        }
                    } else {
                        error!("ping(): Could not extract ping metrics");
                    }
                    debug!("ping(): cmd.status: {}", cmd.status());

                } else {
                    let so: &str = str::from_utf8(&out).unwrap();
                    let se: &str = str::from_utf8(&err).unwrap();
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
            match sender.send(data) {
                Ok(()) => {},
                Err(..) => error!("ping(): Error sending data to worker"),
            }
        }

        let ctrl_msg = ctrl.read().unwrap();
        if ctrl_msg.to_string() != "0" {
            println!("ping(): Stopping due to signal from workers() ({})", ctrl_msg.to_string());
            break;
        }

        //thread::sleep_ms(Duration::from_secs(interval as i64).num_milliseconds() as u32);
        thread::sleep_ms(Duration::from_secs(interval as u64).secs as u32);
    }
    println!("ping(): Done");
}

#[allow(unused_must_use)]
#[cfg(not(target_os = "linux"))]
fn ping(host: String, interval: isize, sender: Sender<HashMap<String, String>>, ctrl: Arc<RwLock<isize>>) {
    println!("ping():{:?}: Starting ({:?}sec) - {:?}", thread::current().name().unwrap(), interval, host);

    loop {
        let mut data: HashMap<String, String> = HashMap::new();
        let mut cmd = Command::new("ping");
        cmd.args(&["-n", "2", "-w", "3", &host]);
        debug!("ping(): cmd: {:?}", cmd);

        // Spawn a process, wait for it to finish, and collect it's output
        match cmd.output() {
            //Err(why) => panic!("Couldn't spawn cmd: {}", why.desc),
            Err(why) => panic!("ping(): Couldn't spawn cmd: {}", why),
            Ok(Output { stderr: err, stdout: out, status: exit }) => {
                //time::now_utc()
                let timespec = time::get_time();
                let ts_ms = timespec.sec + timespec.nsec as i64 / 1000 / 1000;

                // Check if the process succeeded, i.e. the exit code was 0
                if exit.success() {
                    let so: &str = str::from_utf8(&out).unwrap();
                    let re = regex!(r"(?is).*\s\(([\d\.,]+)% loss\).*Minimum = (\d+)ms.*Maximum = (\d+)ms.*Average = (\d+)ms.*");

                    if re.is_match(so) {
                        for cap in re.captures_iter(so) {
                            data.insert("host".to_string(), host.to_string());
                            data.insert("ts".to_string(), ts_ms.to_string());
                            data.insert("loss_pct".to_string(), cap.at(1).unwrap().to_string());
                            data.insert("min".to_string(), cap.at(2).unwrap().to_string());
                            data.insert("max".to_string(), cap.at(3).unwrap().to_string());
                            data.insert("avg".to_string(), cap.at(4).unwrap().to_string());
                        }
                    } else {
                        error!("ping(): Could not extract ping metrics");
                    }
                    debug!("ping(): cmd.status: {:?}", cmd.status());

                } else {
                    let so: &str = str::from_utf8(&out).unwrap();
                    let se: &str = str::from_utf8(&err).unwrap();
                    let re = regex!(r"(?is).*\s\(([\d\.,]+)% loss\).*");

                    if re.is_match(so) {
                        for cap in re.captures_iter(so) {
                            data.insert("host".to_string(), host.to_string());
                            data.insert("ts".to_string(), ts_ms.to_string());
                            data.insert("loss_pct".to_string(), cap.at(1).unwrap().to_string());
                        }
                        warn!("ping(): Failed");
                    } else {
                        error!("ping(): unknown error: {:?}", cmd.status());
                        debug!("ping(): stdout was:\n{}", so);
                        debug!("ping(): stderr was:\n{}", se);
                    }
                }
            },
        }

        debug!("ping(): data = {:?}", data);
        if !data.is_empty() {
            match sender.send(data) {
                Ok(()) => {},
                Err(..) => error!("ping(): Error sending data to worker"),
            }
        }

        let ctrl_msg = ctrl.read().unwrap();
        if ctrl_msg.to_string() != "0" {
            println!("ping(): Stopping due to signal from workers() ({})", ctrl_msg.to_string());
            break;
        }

        thread::sleep_ms(Duration::from_secs(interval as u64).secs as u32);
    }
    println!("ping(): Done");
}


#[allow(unused_must_use)]
fn workers(hosts: &[String], receive_from_main:  Receiver<isize>, send_to_main: Sender<isize>) {
    println!("workers(): Starting - {:?}", hosts);

    let ip_src = net::Ipv4Addr::new(127, 0, 0, 1);
    let ip_dst = net::Ipv4Addr::new(127, 0, 0, 1);
    let addr_src = net::SocketAddrV4::new(ip_src, 8889);
    let addr_dst = net::SocketAddrV4::new(ip_dst, 8889);


    let mut rx_metrics_cnt: isize = 0;
    let ctrl: Arc<RwLock<isize>> = Arc::new(RwLock::new(0isize));
    let (sender_to_ping, receive_from_ping): (Sender<HashMap<String, String>>, Receiver<HashMap<String, String>>) = channel();

    for h in hosts.iter() {
        let sender_to_ping_task = sender_to_ping.clone();
        let ctrl_task = ctrl.clone();
        let h_task = h.clone();

        //thread::Builder::new().name(h_task.to_string()).spawn(move || {
        thread::Builder::new().name(h_task.to_string()).spawn(move || {
            ping(h_task, 5, sender_to_ping_task, ctrl_task);
        });
        //}).detach();
    }

    loop {
        if receive_from_main.try_recv().is_ok() {
            println!("workers(): Stopping due to signal from main()");

            // Send stop to all pings
            println!("workers(): Sending abort to ping()");
            let mut ctrl_w = ctrl.write().unwrap();
            *ctrl_w = 1;

            // Send back to the main the # of received metrics
            match send_to_main.send(rx_metrics_cnt) {
                Ok(()) => {},
                Err(..) => error!("workers(): Error sending data to main: {}", rx_metrics_cnt),
            }
            break;
        }

        let ping_result = receive_from_ping.try_recv();
        if ping_result.is_ok() {
            println!("workers(): ping_result = {:?}", ping_result.unwrap());

            //let message: Vec<u8> = vec![10];
            send_message(net::SocketAddr::V4(addr_src), net::SocketAddr::V4(addr_dst), ping_result.unwrap());

            // Send
            rx_metrics_cnt += 1
        }

        //thread::sleep_ms(Duration::milliseconds(30).num_milliseconds() as u32);
        thread::sleep_ms(Duration::from_secs(30 as u64).secs as u32);
    }

    match send_to_main.send(rx_metrics_cnt) {
        Ok(()) => {},
        Err(..) => error!("workers(): Error sending data to main: {}", rx_metrics_cnt),
    }
    println!("workers(): Done");
}

// Workaround untill SIGKILL can we handheld by Rust in Unix and Windows
fn stop_action() -> bool {
    Path::new("stop.txt").exists()
}




fn socket(listen_on: net::SocketAddr) -> net::UdpSocket {
  let attempt = net::UdpSocket::bind(listen_on);
  let mut socket;
  match attempt {
    Ok(sock) => {
      println!("Bound socket to {}", listen_on);
      socket = sock;
    },
    Err(err) => panic!("Could not bind: {}", err)
  }
  socket
}

pub fn send_message(send_addr: net::SocketAddr, target: net::SocketAddr, data: Vec<u8>) {
  let socket = socket(send_addr);
  println!("Sending data");
  let result = socket.send_to(&data, target);
  drop(socket);
  match result {
    Ok(amt) => println!("Sent {} bytes", amt),
    Err(err) => panic!("Write error: {}", err)
  }
}


#[allow(unused_must_use)]
fn main() {

    let ip_src = net::Ipv4Addr::new(127, 0, 0, 1);
    let ip_dst = net::Ipv4Addr::new(127, 0, 0, 1);
    let addr_src = net::SocketAddrV4::new(ip_src, 8889);
    let addr_dst = net::SocketAddrV4::new(ip_dst, 8889);

    // Read list of hosts
    let path = Path::new("hosts.txt");
    let file = BufReader::new(File::open(path).unwrap());
    let mut hosts: Vec<String> = file.lines().map(Result::unwrap).map(|l| l.trim().to_string()).collect();
    //let mut hosts: Vec<String> = vec!("dns.google.com".to_string(),"localhost".to_string());

    println!("hosts (File): {:?}", hosts);
    println!("main(): Start");
    let mut stop_action_sent: bool = false;
    let (send_from_worker_to_main, receive_from_worker): (Sender<isize>, Receiver<isize>) = channel();
    let (send_from_main_to_worker, receive_from_main): (Sender<isize>, Receiver<isize>) = channel();

    //let message: Vec<u8> = vec![10];
    //send_message(net::SocketAddr::V4(addr_src), net::SocketAddr::V4(addr_dst), message);

    //spawn(move|| {
    //thread::Thread::spawn(move || {
    //}).detach();
    //let guard = thread::Builder::new().name("worker".to_string()).spawn(move || {
    let guard = thread::Builder::new().name("worker".to_string()).scoped(move || {
        workers(&hosts, receive_from_main, send_from_worker_to_main)
    });

    loop {
        let data = receive_from_worker.try_recv();
        if data.is_ok() {
            println!("main(): data = {:?}", data);
            break;
        }
        if stop_action() && !stop_action_sent {
            println!("main(): Sending abort to worker()");

            match send_from_main_to_worker.send(0) {
                Ok(()) => {},
                Err(..) => error!("Workers(): Error sending data to worker: {}", 0),
            }
            stop_action_sent = true;
        }
        //thread::sleep_ms(Duration::seconds(1).num_milliseconds() as u32);
        thread::sleep_ms(Duration::from_secs(1 as u64).secs as u32);
    }

    /*{
    match guard.unwrap().join() {
    //match guard.join() {
        Ok(()) => println!("OK"),
        Err(_) => println!("NOK")
    }
    }*/

    guard.unwrap().join();

    println!("main(): Done");
    process::exit(0);
}

//https://github.com/rust-lang/rust/pull/20615
