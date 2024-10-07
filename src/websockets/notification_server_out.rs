use std::thread;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use std::collections::{HashSet, HashMap};
use std::net::{TcpStream, TcpListener};

use tungstenite::{
    accept,
    protocol::{Role, WebSocket},
    Message,
};

use crate::value_store::stock_information_cache::StockInformationCache;

pub struct NotificationServerOut {
    ip_server: String,
    connection_queue: Arc<RwLock<HashMap::<usize, Vec<String>>>>,
    subscriber_map: Arc<RwLock<HashMap::<String, HashSet<usize>>>>,
    stock_information_cache: Arc<RwLock<StockInformationCache>>,
}

impl NotificationServerOut {
    pub fn new(ip_server: String,
               connection_queue: Arc<RwLock<HashMap::<usize, Vec<String>>>>,
               subscriber_map: Arc<RwLock<HashMap::<String, HashSet<usize>>>>,
               stock_information_cache: Arc<RwLock<StockInformationCache>>) -> Self {
        NotificationServerOut { 
            ip_server: ip_server,
            connection_queue: connection_queue,
            subscriber_map: subscriber_map,
            stock_information_cache: stock_information_cache,
        }
    }

    pub fn start_server(&self) {
        let server = TcpListener::bind(self.ip_server.clone()).unwrap();

        let connection_queue = self.connection_queue.clone();
        let subscriber_map = self.subscriber_map.clone();
        let stock_information_cache = self.stock_information_cache.clone();

        thread::spawn(move || {
            let mut id:usize = 0;

            for stream in server.incoming() {
                let id_cloned = id;
                let connection_queue_cloned = connection_queue.clone();
                let subscriber_map_cloned = subscriber_map.clone();
                let stock_information_cache_cloned = stock_information_cache.clone();

                thread::spawn(move || {
                    let stream_read = stream.unwrap();
                    let send_stream = stream_read.try_clone().unwrap();

                    let websocket_read = match accept(stream_read) {
                        Ok(v) => v,
                        Err(e) => return,
                    };
                    
                    let websocket_send = WebSocket::from_raw_socket(send_stream, Role::Server, None);
        
                    connection_queue_cloned.write().unwrap().insert(id, Vec::new());
                    
                    start_websocket_receiver(
                        websocket_read, connection_queue_cloned.clone(), 
                        subscriber_map_cloned, stock_information_cache_cloned, 
                        id_cloned
                    );
                    
                    start_websocket_sender(
                        websocket_send, 
                        connection_queue_cloned, 
                        id_cloned
                    );
        
                    println!("Spawned websocket {}", id_cloned);
                });

                id += 1;
            }
        });
    }
}

fn start_websocket_receiver(mut receiver: WebSocket<TcpStream>,
                            connection_queue: Arc<RwLock<HashMap::<usize, Vec<String>>>>,
                            subscriber_map: Arc<RwLock<HashMap::<String, HashSet<usize>>>>,
                            stock_information_cache: Arc<RwLock<StockInformationCache>>,
                            id: usize) {
    thread::spawn(move || {
        let mut old_stock:String = String::new();

        loop {
            let message_json:String = match receiver.read() {
                Ok(message) => match message {
                    msg @ Message::Text(_) => msg.into_text().unwrap(),
                    _msg @ Message::Ping(_) | _msg @ Message::Pong(_) => continue,
                    _ => break,
                },
                Err(e) =>{
                    println!("Error in message {} thread: {}", e, id);

                    break;
                },
            };

            let parsed_json:HashMap<String,String> = parse_json(&message_json);

            if !parsed_json.contains_key("stock") {
                println!("Error with stock in thread {}", id);

                continue;
            }

            match subscriber_map.write().unwrap().get_mut(&old_stock) {
                Some(v) => { v.remove(&id); },
                None => println!("Couldn't find key {:?}", &old_stock),
            };

            let stock_name:String = parsed_json.get("stock").unwrap().to_string();

            println!("Received Stockname {}", stock_name);

            if &stock_name[..] != "*" && !stock_information_cache.read().unwrap().has_key(&stock_name) {
                println!("Couldn't find key {:?}", old_stock);

                continue;
            }

            let key_is_there = subscriber_map.read().unwrap().contains_key(&stock_name);

            match key_is_there {
                true => { subscriber_map.write().unwrap().get_mut(&stock_name).unwrap().insert(id); },
                false => { subscriber_map.write().unwrap().insert(stock_name.clone(), HashSet::from([id])); },
            };
            
            if &stock_name[..] == "*" {
                connection_queue.write().unwrap().insert(id, stock_information_cache.read().unwrap().get_entire_cache());
            }

            old_stock = stock_name;
        }

        println!("Closing Receiver thread {}", id);

        match subscriber_map.write().unwrap().get_mut(&old_stock) {
            Some(v) => { v.remove(&id); },
            None => (),
        };
    });
}

fn start_websocket_sender(mut sender: WebSocket<TcpStream>,
                   connection_queue: Arc<RwLock<HashMap::<usize, Vec<String>>>>,
                   id: usize) {
    thread::spawn(move || {
        let mut ping_cnt:usize = 0;

        loop {
            if connection_queue.read().unwrap().len() > 1000 { 
                continue; 
            }

            let connection_vec = match connection_queue.read().unwrap().get(&id) {
                Some(v) => v.clone(),
                None => break,
            };

            if connection_vec.len() == 0 {
                thread::sleep(Duration::from_millis(10));

                if !send_ping(&mut sender, &mut ping_cnt) { 
                    break; 
                }
                
                continue;
            }

            match connection_queue.write().unwrap().get_mut(&id) {
                Some(v) => v.clear(),
                None => break,
            };

            for update in connection_vec.into_iter() {
                match sender.send(Message::Text(update)) {
                    Ok(v) => v,
                    Err(_) => break,
                }
            }
        }

        println!("Error sending message. Closing Websocket {}", id);
        connection_queue.write().unwrap().remove(&id);
    });

}

fn send_ping(sender: &mut WebSocket<TcpStream>, ping_cnt: &mut usize) -> bool {
    *ping_cnt += 1;

    if *ping_cnt < 100 { 
        return true; 
    }

    match sender.send(Message::Ping(Vec::new())) {
        Ok(v) => v,
        Err(_) =>  return false,
    };

    *ping_cnt = 0;

    true
}

pub fn parse_json(json_data: &str) -> HashMap<String ,String> {
    let mut tmp: String = String::new();
    let mut key: String = String::new();

    let mut parsed_json:HashMap<String,String> = HashMap::new();

    for p in json_data.chars() {
        if p == ' ' || p == '\n' || p == '\t' || p == '\"' || p == '{' || p == '}' { continue; }
        
        if p == ':' || p == ',' {
            match key.len() {
                0 => key = tmp,
                _ => {
                    parsed_json.insert(key, tmp);
                    key = String::new();
                }
            };
            
            tmp = String::new();

            continue;
        }

        tmp.push(p);
    }

    if key.len() > 0 && tmp.len() > 0 { parsed_json.insert(key, tmp); } 

    parsed_json
}