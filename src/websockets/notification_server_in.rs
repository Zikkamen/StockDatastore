use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap};
use std::net::TcpListener;

use tungstenite::{
    Message,
    accept,
};

use crate::value_store::stock_information_cache::StockInformationCache;

pub struct NotificationServerIn {
    ip_server: String,
    connection_queue: Arc<RwLock<HashMap::<usize, Vec<String>>>>,
    subscriber_map: Arc<RwLock<HashMap::<String, HashSet<usize>>>>,
    stock_information_cache: Arc<RwLock<StockInformationCache>>,
}

impl NotificationServerIn {
    pub fn new(ip_server: String,
               connection_queue: Arc<RwLock<HashMap::<usize, Vec<String>>>>,
               subscriber_map: Arc<RwLock<HashMap::<String, HashSet<usize>>>>,
               stock_information_cache: Arc<RwLock<StockInformationCache>>) -> Self {
        NotificationServerIn {
            ip_server: ip_server, 
            connection_queue: connection_queue, 
            subscriber_map: subscriber_map,
            stock_information_cache: stock_information_cache,
        }
    }

    pub fn start_server(&mut self) {
        let server = TcpListener::bind(self.ip_server.clone()).unwrap();

        for stream in server.incoming() {
            let stream = match stream {
                Ok(v) => v,
                Err(_) => continue,
            };

            let mut websocket = match accept(stream) {
                Ok(v) => v,
                Err(_) => continue,
            };

            let _ = websocket.send(Message::Text(self.stock_information_cache.read().unwrap().get_stock_names()));

            loop {
                let message = match websocket.read() {
                    Ok(p) => p,
                    Err(e) => {
                        println!("Error receiving message {} \n Closing Client", e);
                        
                        break;
                    },
                };

                match message {
                    msg @ Message::Text(_) => {
                        let text: String = match msg.into_text() {
                            Ok(v) => v,
                            Err(_) => continue,
                        };

                        let name = self.stock_information_cache.write().unwrap().add_json(&text);
    
                        let mut ids_to_update:HashSet<usize> = HashSet::new();
    
                        match self.subscriber_map.read().unwrap().get(&name){
                            Some(list_of_ids) => {
                                for id in list_of_ids.iter() {
                                    ids_to_update.insert(*id);
                                }
                            },
                            None => (),
                        }
                        
                        match self.subscriber_map.read().unwrap().get("*") {
                            Some(list_of_ids) => {
                                for id in list_of_ids.iter() {
                                    ids_to_update.insert(*id);
                                }
                            },
                            None => (),
                        }
    
                        let mut connection_vec = self.connection_queue.write().unwrap();
    
                        for id in ids_to_update.iter() {
                            match connection_vec.get_mut(id) {
                                Some(v) => v.push(text.clone()),
                                None => continue,
                            };
                        }
                    }
                    _ => (),
                }
            }
        }
    }
}