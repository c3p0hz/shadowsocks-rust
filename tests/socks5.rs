#![allow(dead_code, unused_imports)]

extern crate shadowsocks;
extern crate tokio_core;
extern crate futures;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::thread;
use std::net::SocketAddr;
use std::sync::{Arc, Barrier};
use std::time::Duration;
use std::io::Cursor;

use tokio_core::reactor::Core;
use tokio_core::io::{read_to_end, write_all, flush};
use futures::Future;

use shadowsocks::relay::tcprelay::client::Socks5Client;
use shadowsocks::config::{Config, ServerConfig};
use shadowsocks::crypto::CipherType;
use shadowsocks::relay::socks5::Address;
use shadowsocks::{run_local, run_server};

const SERVER_ADDR: &'static str = "127.0.0.1:8092";
const LOCAL_ADDR: &'static str = "127.0.0.1:8290";

const PASSWORD: &'static str = "test-password";
const METHOD: CipherType = CipherType::Aes128Cfb;

fn get_config() -> Config {
    let mut cfg = Config::new();
    cfg.local = Some(LOCAL_ADDR.parse().unwrap());
    cfg.server = vec![ServerConfig::basic(SERVER_ADDR.parse().unwrap(), PASSWORD.to_owned(), METHOD)];
    cfg
}

fn get_client_addr() -> SocketAddr {
    LOCAL_ADDR.parse().unwrap()
}

fn start_server(bar: Arc<Barrier>) {
    thread::spawn(move || {
        drop(env_logger::init());
        bar.wait();
        run_server(get_config()).unwrap();
    });
}

fn start_local(bar: Arc<Barrier>) {
    thread::spawn(move || {
        drop(env_logger::init());
        bar.wait();
        run_local(get_config()).unwrap();
    });
}

#[test]
fn socks5_relay() {
    drop(env_logger::init());

    let bar = Arc::new(Barrier::new(3));

    start_server(bar.clone());
    start_local(bar.clone());

    bar.wait();

    // Wait until all server starts
    thread::sleep(Duration::from_secs(1));

    let mut lp = Core::new().unwrap();
    let handle = lp.handle();

    let c = Socks5Client::connect(Address::DomainNameAddress("www.example.com".to_owned(), 80),
                                  get_client_addr(),
                                  handle);
    let fut = c.and_then(|c| {
        let req = b"GET / HTTP/1.0\r\nHost: www.example.com\r\nAccept: */*\r\n\r\n";
        write_all(c, req.to_vec())
            .and_then(|(c, _)| flush(c))
            .and_then(|c| read_to_end(c, Vec::new()))
            .map(|(_, buf)| {
                println!("Got reply from server: {}", String::from_utf8(buf).unwrap());
            })
    });

    lp.run(fut).unwrap();
}