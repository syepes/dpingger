//RUST_LOG=debug
#![feature(slicing_syntax)]

extern crate time;

use std::io::BufferedReader;
use std::io::File;
use std::io::fs::PathExtensions;

use std::collections::HashMap;

use std::io::Timer;
use std::time::Duration;

use std::task::spawn;
use std::comm::{channel, Sender, Receiver};
use std::sync::{Arc, RWLock};



#[allow(unused_must_use)]
fn ping<'a>(host: &'a String, interval: int, sender: Sender<HashMap<String, String>>, ctrl: Arc<RWLock<int>>) {
    let mut timer = Timer::new().unwrap();
    println!("ping(): Starting ({}sec)", interval);

    loop {
        let mut data: HashMap<String, String> = HashMap::new();
        let timespec = time::get_time();
        let ts_ms = timespec.sec + timespec.nsec as i64 / 1000 / 1000;

        data.insert("host".to_string(), host.to_string());
        data.insert("ts".to_string(), ts_ms.to_string());

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
fn workers<'a>(hosts: &'a [String], receive_from_main:  Receiver<int>, send_to_main: Sender<int>) {
    let ctrl: Arc<RWLock<int>> = Arc::new(RWLock::new(0i));
    let (sender_to_ping, receive_from_ping): (Sender<HashMap<String, String>>, Receiver<HashMap<String, String>>) = channel();

    let mut rx_metrics_cnt: int = 0;
    let mut timer = Timer::new().unwrap();

    for h in hosts.iter() {
        println!("{}",h)
        let sender_to_ping_task = sender_to_ping.clone();
        let ctrl_local = ctrl.clone();

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
    // Read list of nodes
    let path = Path::new("nodes.txt");
    let mut file = BufferedReader::new(File::open(&path));
    let mut hosts: Vec<String> = file.read_to_string().unwrap().lines().map(|l| l.trim().to_string()).collect();
    //let mut hosts: Vec<String> = vec!("dns.google.com".to_string(),"localhost".to_string());


    println!("hosts (File): {}", hosts);

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

