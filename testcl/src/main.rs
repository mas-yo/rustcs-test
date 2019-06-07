use lazy_static::lazy_static;
use rand;
use rand::Rng;
use std::env;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use tokio::codec::{Decoder, Encoder, Framed};
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use uuid::Uuid;
use std::time::*;

mod command;

use crate::command::*;

lazy_static! {
    static ref CONNS_COUNT: Arc<Mutex<u32>> = { Arc::new(Mutex::new(0u32)) };
}
lazy_static! {
    static ref TIME_SUM: Arc<Mutex<u128>> = { Arc::new(Mutex::new(0u128)) };
}

fn start_client(
    addr: String,
) {
    let addr = format!("{}:29180", addr).parse().unwrap();
//    let name2 = name.clone();

    // let name:String = Uuid::new_v4().to_hyphenated().to_string();
    // let bye_text = format!("bye {}", name);

    //    let name = format!("{}", name);
    let client = TcpStream::connect(&addr)
        .and_then(move |stream| {
            println!("connected");
            let framed = Framed::new(stream, Codec::default());
            let (tx, rx) = framed.split();

            let mut tx = tx.wait();
            tx.send(C2S::InputText("testcl".to_string()));
            tx.flush();

            let mut start_at = Option::<Instant>::None;
            let mut count = 0;
            let mut time = 0;
            let receive = rx
                .for_each(move |cmd| {
                    // println!("recv {:?}", cmd);
                    match cmd {
                        S2C::ObjList => {
                            // dbg!(start_at);
                            if let Some(at) = start_at.as_ref() {
                                let end = at.elapsed();
                                count = count + 1;
                                time = time + end.as_millis();

                                if count > 10000 {
                                    println!("time: {}", time);
                                    count = 0;
                                    time = 0;
                                }
                            }
                            // else {
                            //     println!("?");
                            // }
                            start_at = Some(Instant::now());
                            // dbg!(start_at);
                            tx.send(C2S::InputText("testcl".to_string()));
                            tx.flush();

                            //time
                        }
                        _ => {}
                    }
                    Ok(())
                })
                .map_err(|_| ());

            tokio::spawn(receive);
            Ok(())
        })
        .map_err(|err| {
            println!("connection error = {:?}", err);
        });

    println!("start client");
    tokio::run(client);
}

pub fn main() -> Result<(), Box<std::error::Error>> {

    let s = start_client("192.168.32.243".to_string());
    // let args: Vec<String> = env::args().collect();
    // let addr = args[1].clone();

    // let (tx, rx) = futures::sync::mpsc::channel::<(u32, String)>(1024);
    // let tx2 = tx.clone();

    // let mut ids = Vec::new();
    // for i in 0..100 {
    //     ids.push(format!("user{:>04}", i));
    // }

    // let mut wait = tx.wait();
    // for i in 0..49 {
    //     if let Err(e) = wait.send((i, ids[i as usize].clone())) {
    //         println!("first send room err {}", e);
    //     }
    // }
    // // .map_err(|_|());

    // let report_time = Ok(()).into_future().and_then(|_| {
    //     let task = tokio::timer::Interval::new(
    //         std::time::Instant::now(),
    //         std::time::Duration::from_secs(10),
    //     )
    //     .for_each(|_| {
    //         let time = TIME_SUM.lock().unwrap();
    //         let count = CONNS_COUNT.lock().unwrap();
    //         println!("avg time: {}", *time as f64 / *count as f64);
    //         Ok(())
    //     })
    //     .map_err(|_| ());
    //     // tokio::spawn(task);
    //     Ok(())
    // });
    // {
    //     let start = rx
    //         .for_each(move |(id, name)| {
    //             start_client(addr.clone(), id, name, tx2.clone());
    //             Ok(())
    //         })
    //         .and_then(|_| {
    //             println!("END");
    //             Ok(())
    //         });
    //     tokio::run(report_time.and_then(|_| start));
    // }

    Ok(())
}
