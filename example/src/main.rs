use rust_sfp as sfp;
use rust_sfp::FrameWriter;
use std::thread;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time;

#[cfg(windows)]
const ADDR: &str = "127.0.0.1:10000";
#[cfg(unix)]
const ADDR: &str = "unix:/tmp/rust_sfp_example.sock";

type ID = u128;

struct Clients{
    clients: HashMap<ID, sfp::ConnectionWriter>,
    next_id: ID,
}

impl Clients{
    pub fn new() -> Self{
        Self{clients: HashMap::new(), next_id: 0}
    }
    pub fn add(&mut self, writer: sfp::ConnectionWriter) -> ID{
        let id = self.next_id;
        self.next_id += 1;
        self.clients.insert(id, writer);
        id
    }
    pub fn remove(&mut self, id: ID){
        self.clients.remove(&id);
    }
    pub fn send(&mut self, mut frame: Vec<u8>){
        let mut to_remove: Vec<ID> = Vec::new();
        for (id, writer) in self.clients.iter_mut(){
            if let Err(_) = writer.write_frame(&mut frame){
                to_remove.push(*id)
            } else {
                println!("Sent frame to {}", id);
            }
        }
        to_remove.iter().for_each(|id|{
            self.remove(*id);
        })
    }
}

fn server(){
    let server = sfp::Server::bind(&ADDR.parse().unwrap()).unwrap();
    let clients = Arc::new(Mutex::new(Clients::new()));
    for (connection, addr) in server{
        let (reader, writer) = connection.separate().unwrap();
        let id = {
            let mut cl = clients.lock().unwrap();
            cl.add(writer)
        };
        println!("New connection from {} with id {}", addr, id);
        let clients = clients.clone();
        thread::spawn(move || {
            for frame in reader{
                println!("Recv frame from {}", id);
                let mut cl = clients.lock().unwrap();
                cl.send(frame);
            }
            let mut cl = clients.lock().unwrap();
            cl.remove(id);
            println!("Connection {} closed", id);
        });
    }
}

fn client(msg: String){
    let msg_frame = msg.into_bytes();
    let connection = sfp::Connection::connect(&ADDR.parse().unwrap()).unwrap();
    println!("Connected");
    let (reader, mut writer) = connection.separate().unwrap();
    thread::spawn(move || {
        for frame in reader{
            println!("Received {} from server", String::from_utf8(frame).unwrap());
        }
    });
    loop {
        if let Err(_) = writer.write_frame(&mut msg_frame.clone()){break}
        if let Err(_) = writer.flush(){break}
        println!("Sent frame to server");
        thread::sleep(time::Duration::from_secs(1));
    }
    println!("Connection closed");
}

fn main() {
    let target = std::env::args().nth(1).unwrap_or("server".parse().unwrap());
    let msg = std::env::args().nth(2).unwrap_or("A".parse().unwrap());
    if target == "client" {
        client(msg);
    } else {
        server();
    }
}
