use std::thread;
use std::sync::RwLock;
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;

use websocket::sync::Server;
use websocket::{OwnedMessage, Message};
use websocket::server::upgrade::WsUpgrade;

mod database_clients;
mod value_store;
mod file_reader;
mod data_services;

use crate::data_services::data_service::DataService;
use crate::value_store::credentials_store::CredentialsStore;
use crate::data_services::data_service::StockData;

fn main() {
    let credentials_store = CredentialsStore::new();
    let mut data_service = DataService::new(credentials_store);

    let connection_queue = Arc::new(RwLock::new(HashMap::<usize, Vec<String>>::new()));
    let current_data = Arc::new(RwLock::new(data_service.get_stock_data_copy()));

    start_websocketserver(Arc::clone(&connection_queue), Arc::clone(&current_data));

    let server = Server::bind("127.0.0.1:9003").unwrap();

    for connection in server.filter_map(Result::ok) {
        let mut client = connection.accept().unwrap();
        let (mut receiver, mut sender) = client.split().unwrap();

        for message in receiver.incoming_messages(){
            let message:OwnedMessage = match message {
                Ok(p) => p,
                Err(e) => {
                    println!("Error receiving message {} \n Closing Client", e);
                    let _ = sender.send_message(&Message::close());
                    break;
                },
            };

            match message {
                OwnedMessage::Text(txt) => {
                    let text: String = txt.parse().unwrap();

                    let iter_keys:Vec<usize> = connection_queue.read().unwrap().keys().copied().collect();
                    let mut connection_vec = connection_queue.write().unwrap();
    
                    for key in iter_keys.iter() {
                        match connection_vec.get_mut(key) {
                            Some(v) => if v.len() < 1000 {
                                v.push(text.clone());
                            },
                            None => continue,
                        };
                    }
                }
                OwnedMessage::Close(_) => {
                    let _ = sender.send_message(&Message::close());
                    break;
                }
                OwnedMessage::Ping(data) => {
                    sender.send_message(&OwnedMessage::Pong(data)).unwrap();
                }
                _ => (),
            }
        }
    }
}

fn stockdata_to_json(update: StockData) -> String {
    format!("{{
            \"name\": \"{}\", 
            \"avg_price\": {}, 
            \"min_price\": {}, 
            \"max_price\": {}, 
            \"volume_moved\": {}, 
            \"num_of_trades\": {}, 
            \"time\": {}
        }}",
        update.name,
        update.avg_price,
        update.min_price,
        update.max_price,
        update.volume_moved,
        update.num_of_trades,
        update.time,
    )
}

fn start_websocketserver(connection_queue: Arc<RwLock<HashMap::<usize, Vec<String>>>>, current_data: Arc<RwLock<Vec<StockData>>>){
    let server = Server::bind("localhost:9002").unwrap();

    thread::spawn(move || {
        let mut id:usize = 0;

        for connection in server.filter_map(Result::ok) {
            connection_queue.write().unwrap().insert(
                id, 
                current_data.read().unwrap().iter().map(|data| stockdata_to_json(data.clone())).collect()
            );

            start_websocket(connection, Arc::clone(&connection_queue), id);

            println!("Spawned websocket {}", id);
            id += 1;
        }
    });
}

fn start_websocket(connection: WsUpgrade<std::net::TcpStream, Option<websocket::server::upgrade::sync::Buffer>>,
        connection_queue: Arc<RwLock<HashMap::<usize, Vec<String>>>>,
        id: usize) {
    let client = connection.accept().unwrap();
    let (_receiver, mut sender) = client.split().unwrap();

    thread::spawn(move || {
        let thread_id = id;
        let mut ping_cnt:usize = 0;

        loop {
            let connection_vec = match connection_queue.read().unwrap().get(&thread_id) {
                Some(v) => v.clone(),
                None => panic!("Error retrieving id {}. Closing Websocket.", thread_id),
            };

            if connection_vec.len() == 0 {
                thread::sleep(Duration::from_millis(100));

                ping_cnt += 1;

                if ping_cnt == 100 {
                    match sender.send_message(&OwnedMessage::Ping(thread_id.to_string().as_bytes().to_vec())) {
                        Ok(v) => v,
                        Err(e) => { 
                            println!("Error sending message {}. Closing Websocket {}", e, thread_id);
                            
                            connection_queue.write().unwrap().remove(&thread_id);
                            
                            return;
                        },
                    }

                    ping_cnt = 0;
                }
                
                continue;
            }

            match connection_queue.write().unwrap().get_mut(&thread_id) {
                Some(v) => v.clear(),
                None => panic!("Error retrieving id {}. Closing Websocket.", thread_id),
            };

            for update in connection_vec.iter() {
                println!("This is the update {} {}", thread_id, update.clone());

                match sender.send_message(&OwnedMessage::Text(update.to_string())) {
                    Ok(v) => v,
                    Err(e) => { 
                        println!("Error sending message {}. Closing Websocket {}", e, thread_id); 
                        
                        connection_queue.write().unwrap().remove(&thread_id);
                        
                        return;
                    },
                }
            }
        }
    });
}