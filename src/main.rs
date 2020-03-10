//RUST_LOG=debug
#![allow(unused_mut, unused_variables)]
#![feature(plugin)]
// https://github.com/ujh/iomrascalai/blob/ee1af121ec67a85701e5533f02f66e4cd37083ff/src/main.rs
// http://stackoverflow.com/questions/28490170/entry-point-could-not-be-located-when-running-program-on-windows

#[macro_use]
extern crate log;
extern crate env_logger;

use regex::Regex;
use std::fs::File;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

//http://stackoverflow.com/questions/29216271/creating-a-vector-of-strings-using-the-new-stdfsfile
//https://github.com/phildawes/racer/blob/master/src/racer/util.rs#L12
use std::io::{BufRead, BufReader};
use std::str;
use std::thread;
//use std::thread::{Thread,JoinGuard};
use std::collections::HashMap;
use std::net;
use std::result::Result;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, RwLock};

#[allow(unused_must_use)]
#[cfg(target_os = "linux")]
fn ping(
  host: String,
  interval: isize,
  sender: Sender<HashMap<String, String>>,
  ctrl: Arc<RwLock<isize>>,
) {
  println!(
    "ping():{}: Starting ({}sec) - {}",
    thread::current().name().unwrap(),
    interval,
    host
  );

  loop {
    let mut data: HashMap<String, String> = HashMap::new();
    let mut cmd = Command::new("ping");
    cmd.args(&["-nqc", "2", "-w", "3", &host]);
    debug!("ping(): cmd: {:?}", cmd);

    // Spawn a process, wait for it to finish, and collect it's output
    match cmd.output() {
      Err(why) => panic!("ping(): Couldn't spawn cmd: {} - {}", why, &why.to_string()),
      Ok(Output {
        stderr: err,
        stdout: out,
        status: exit,
      }) => {
        // let ts = time::get_time();
        let start = SystemTime::now();
        let ts = start
          .duration_since(UNIX_EPOCH)
          .expect("Time went backwards");

        // Check if the process succeeded, i.e. the exit code was 0
        if exit.success() {
          let so: &str = str::from_utf8(&out).unwrap();
          let re = Regex::new(
            r"(?is).*\s([\d\.,]+)% packet loss.*= ([\d\.,]+)/([\d\.,]+)/([\d\.,]+)/([\d\.,]+) ms.*",
          )
          .expect("Invalid regex");

          if re.is_match(so) {
            for cap in re.captures_iter(so) {
              data.insert("host".to_string(), host.to_string());
              // data.insert("ts".to_string(), ts.sec.to_string());
              data.insert("ts".to_string(), ts.as_millis().to_string());
              data.insert("loss_pct".to_string(), cap[1].to_string());
              data.insert("max".to_string(), cap[2].to_string());
              data.insert("min".to_string(), cap[3].to_string());
              data.insert("avg".to_string(), cap[4].to_string());
            }
          } else {
            error!("ping(): Could not extract ping metrics");
          }
          debug!(
            "ping(): cmd.status: {:?} - {:?}",
            cmd.status().unwrap(),
            exit
          );
        } else {
          let so: &str = str::from_utf8(&out).unwrap();
          let se: &str = str::from_utf8(&err).unwrap();
          let re = Regex::new(r"(?is).*\s([\d\.,]+)% packet loss.*").expect("Invalid regex");

          if re.is_match(so) {
            for cap in re.captures_iter(so) {
              data.insert("host".to_string(), host.to_string());
              data.insert("ts".to_string(), ts.as_millis().to_string());
              data.insert("loss_pct".to_string(), cap[1].to_string());
            }
            warn!("ping(): Failed");
          } else {
            error!("ping(): unknown error: {:?}", cmd.status().unwrap());
            debug!("ping(): stdout was:\n{}", so);
            debug!("ping(): stderr was:\n{}", se);
          }
        }
      }
    }

    debug!("ping(): data = {:?}", data);
    if !data.is_empty() {
      match sender.send(data) {
        Err(..) => error!("ping(): Error sending data to worker"),
        Ok(()) => {}
      }
    }

    let ctrl_msg = ctrl.read().unwrap();
    if ctrl_msg.to_string() != "0" {
      println!(
        "ping(): Stopping due to signal from workers() ({})",
        ctrl_msg.to_string()
      );
      break;
    }

    thread::sleep(Duration::from_secs(interval as u64));
  }
  println!("ping(): Done");
}

#[allow(unused_must_use)]
#[cfg(not(target_os = "linux"))]
fn ping(
  host: String,
  interval: isize,
  sender: Sender<HashMap<String, String>>,
  ctrl: Arc<RwLock<isize>>,
) {
  println!(
    "ping():{:?}: Starting ({:?}sec) - {:?}",
    thread::current().name().unwrap(),
    interval,
    host
  );

  loop {
    let mut data: HashMap<String, String> = HashMap::new();
    let mut cmd = Command::new("ping");
    cmd.args(&["-n", "2", "-w", "3", &host]);
    debug!("ping(): cmd: {:?}", cmd);

    // Spawn a process, wait for it to finish, and collect it's output
    match cmd.output() {
      Err(why) => panic!(
        "ping(): Couldn't spawn cmd: {} - {}",
        why,
        Error::description(&why)
      ),
      Ok(Output {
        stderr: err,
        stdout: out,
        status: exit,
      }) => {
        // let ts = Time::now().to_timespec();
        let start = SystemTime::now();
        let ts = start
          .duration_since(UNIX_EPOCH)
          .expect("Time went backwards");

        // Check if the process succeeded, i.e. the exit code was 0
        if exit.success() {
          let so: &str = str::from_utf8(&out).unwrap();
          let re = regex!(
            r"(?is).*\s\(([\d\.,]+)% loss\).*Minimum = (\d+)ms.*Maximum = (\d+)ms.*Average = (\d+)ms.*"
          );

          if re.is_match(so) {
            for cap in re.captures_iter(so) {
              data.insert("host".to_string(), host.to_string());
              data.insert("ts".to_string(), ts.as_millis().to_string());
              data.insert("loss_pct".to_string(), cap.at(1).unwrap().to_string());
              data.insert("min".to_string(), cap.at(2).unwrap().to_string());
              data.insert("max".to_string(), cap.at(3).unwrap().to_string());
              data.insert("avg".to_string(), cap.at(4).unwrap().to_string());
            }
          } else {
            error!("ping(): Could not extract ping metrics");
          }
          debug!(
            "ping(): cmd.status: {:?} : {:?}",
            cmd.status().unwrap(),
            exit
          );
        } else {
          let so: &str = str::from_utf8(&out).unwrap();
          let se: &str = str::from_utf8(&err).unwrap();
          let re = regex!(r"(?is).*\s\(([\d\.,]+)% loss\).*");

          if re.is_match(so) {
            for cap in re.captures_iter(so) {
              data.insert("host".to_string(), host.to_string());
              data.insert("ts".to_string(), ts.sec.to_string());
              data.insert("loss_pct".to_string(), cap.at(1).unwrap().to_string());
            }
            warn!("ping(): Failed");
          } else {
            error!("ping(): unknown error: {:?}", cmd.status().unwrap());
            debug!("ping(): stdout was:\n{}", so);
            debug!("ping(): stderr was:\n{}", se);
          }
        }
      }
    }

    debug!("ping(): data = {:?}", data);
    if !data.is_empty() {
      match sender.send(data) {
        Err(..) => error!("ping(): Error sending data to worker"),
        Ok(()) => {}
      }
    }

    let ctrl_msg = ctrl.read().unwrap();
    if ctrl_msg.to_string() != "0" {
      println!(
        "ping(): Stopping due to signal from workers() ({})",
        ctrl_msg.to_string()
      );
      break;
    }

    thread::sleep(Duration::from_secs(interval as u64));
  }
  println!("ping(): Done");
}

#[allow(unused_must_use)]
fn workers(hosts: &[String], receive_from_main: Receiver<isize>, send_to_main: Sender<isize>) {
  println!("workers(): Starting - {:?}", hosts);

  let ip_src = net::Ipv4Addr::new(127, 0, 0, 1);
  let ip_dst = net::Ipv4Addr::new(127, 0, 0, 1);
  let addr_src = net::SocketAddrV4::new(ip_src, 8889);
  let addr_dst = net::SocketAddrV4::new(ip_dst, 8889);

  let mut rx_metrics_cnt: isize = 0;
  let ctrl: Arc<RwLock<isize>> = Arc::new(RwLock::new(0isize));
  let (sender_to_ping, receive_from_ping): (
    Sender<HashMap<String, String>>,
    Receiver<HashMap<String, String>>,
  ) = channel();

  for h in hosts.iter() {
    let sender_to_ping_task = sender_to_ping.clone();
    let ctrl_task = ctrl.clone();
    let h_task = h.clone();

    //thread::Builder::new().name(h_task.to_string()).scoped(move || {
    thread::Builder::new()
      .name(h_task.to_string())
      .spawn(move || {
        ping(h_task, 5, sender_to_ping_task, ctrl_task);
      });
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
        Err(..) => error!("workers(): Error sending data to main: {}", rx_metrics_cnt),
        Ok(()) => {}
      }
      break;
    }

    let ping_result = receive_from_ping.try_recv();
    if ping_result.is_ok() {
      println!("workers(): ping_result = {:?}", ping_result);

      //let message: Vec<u8> = vec![10];
      //send_message(net::SocketAddr::V4(addr_src), net::SocketAddr::V4(addr_dst), ping_result.unwrap());
      //send_message(net::SocketAddr::V4(addr_src), net::SocketAddr::V4(addr_dst), msg);
      //for (k, v) in ping_result.unwrap().iter() { println!("k/v = {:?}/{:?}", k.to_string(), v.to_string()); }

      // Send
      rx_metrics_cnt += 1
    }

    thread::sleep(Duration::from_secs(10 as u64));
  }

  match send_to_main.send(rx_metrics_cnt) {
    Err(..) => error!("workers(): Error sending data to main: {}", rx_metrics_cnt),
    Ok(()) => {}
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
    Err(why) => panic!("Could not bind: {} - {}", why, &why.to_string()),
    Ok(sock) => {
      println!("Bound socket to {}", listen_on);
      socket = sock;
    }
  }
  socket
}

pub fn send_message(send_addr: net::SocketAddr, target: net::SocketAddr, data: Vec<u8>) {
  let socket = socket(send_addr);
  println!("Sending data");
  let result = socket.send_to(&data, target);
  drop(socket);
  match result {
    Err(why) => panic!("Write error: {} - {}", why, &why.to_string()),
    Ok(amt) => println!("Sent {} bytes", amt),
  }
}

#[allow(unused_must_use)]
fn main() {
  env_logger::init();

  let ip_src = net::Ipv4Addr::new(127, 0, 0, 1);
  let ip_dst = net::Ipv4Addr::new(127, 0, 0, 1);
  let addr_src = net::SocketAddrV4::new(ip_src, 8889);
  let addr_dst = net::SocketAddrV4::new(ip_dst, 8889);

  // Read list of hosts
  let path = Path::new("hosts.txt");
  let file = match File::open(&path) {
    Err(why) => panic!(
      "Couldn't open {} file - {}",
      path.display(),
      &why.to_string()
    ),
    Ok(file) => BufReader::new(file),
  };

  let mut hosts: Vec<String> = file
    .lines()
    .map(Result::unwrap)
    .map(|l| l.trim().to_string())
    .collect();
  //let mut hosts: Vec<String> = vec!("dns.google.com".to_string(),"localhost".to_string());

  println!("hosts (File): {:?}", hosts);
  println!("main(): Start");
  let mut stop_action_sent: bool = false;
  let (send_from_worker_to_main, receive_from_worker): (Sender<isize>, Receiver<isize>) = channel();
  let (send_from_main_to_worker, receive_from_main): (Sender<isize>, Receiver<isize>) = channel();

  //let message: Vec<u8> = vec![10];
  //send_message(net::SocketAddr::V4(addr_src), net::SocketAddr::V4(addr_dst), message);

  // http://stackoverflow.com/questions/36703806/when-should-i-use-stdthreadbuilder-instead-of-stdthreadspawn
  let guard = thread::Builder::new()
    .name("worker".to_string())
    .spawn(move || workers(&hosts, receive_from_main, send_from_worker_to_main));

  loop {
    let data = receive_from_worker.try_recv();
    if data.is_ok() {
      println!("main(): data = {:?}", data);
      break;
    }
    if stop_action() && !stop_action_sent {
      println!("main(): Sending abort to worker()");

      match send_from_main_to_worker.send(0) {
        Err(..) => error!("Workers(): Error sending data to worker: {}", 0),
        Ok(()) => {}
      }
      stop_action_sent = true;
    }
    thread::sleep(Duration::from_secs(1 as u64));
  }

  match guard.unwrap().join() {
    Err(why) => panic!("main(): Error {:?}", why),
    Ok(()) => println!("main(): Done"),
  }
}
