use std::thread;
use std::sync::RwLock;
use std::sync::Arc;
use std::time::Duration;

use websocket::sync::Server;
use websocket::{OwnedMessage};
use websocket::sync::Writer;
use websocket::stream::sync::TcpStream;

mod database_clients;
mod value_store;
mod file_reader;
mod data_services;

use crate::data_services::data_service::DataService;
use crate::value_store::credentials_store::CredentialsStore;

fn main() {
    let connection_queue = Arc::new(RwLock::new(Vec::<Vec<String>>::new()));
    let connection_queue_clone = Arc::clone(&connection_queue);



    let credentials_store = CredentialsStore::new();
    let mut data_service = DataService::new(credentials_store);

    start_websocketserver(connection_queue_clone);

    let mut current_in_loop: usize = 0;

    loop {
        thread::sleep(Duration::from_millis(1000));

        let updated_data = data_service.check_for_updates();
        let mut connection_vec = connection_queue.write().unwrap();

        for update in updated_data {
            let update_string = format!("{{\"name\": \"{}\", \"avg_price\": {}, \"min_price\": {}, \"max_price\": {}, \"num_of_trades\": {}, \"time\": {}}}",
                update.name,
                update.avg_price,
                update.min_price,
                update.max_price,
                update.num_of_trades,
                update.time,
            );

            for i in 0..current_in_loop {
                connection_vec[i].push(update_string.clone());
            }
        }

        let n = connection_vec.len();

        if n == current_in_loop{ continue; }

        let current_list = data_service.get_stock_data_copy();

        for i in current_in_loop..n {
            for update in &current_list {
                let update_string = format!("{{\"name\": \"{}\", \"avg_price\": {}, \"min_price\": {}, \"max_price\": {}, \"num_of_trades\": {}, \"time\": {}}}",
                    update.name,
                    update.avg_price,
                    update.min_price,
                    update.max_price,
                    update.num_of_trades,
                    update.time,
                );

                connection_vec[i].push(update_string.clone());
            }
        }

        current_in_loop = n;
    }
}

fn start_websocketserver(connection_queue: Arc<RwLock<Vec::<Vec<String>>>>){
    let server = Server::bind("127.0.0.1:9002").unwrap();

    thread::spawn(move || {
        let mut id = 0;

        for connection in server.filter_map(Result::ok) {
            let client = connection.accept().unwrap();
            let (_receiver, mut sender) = client.split().unwrap();

            connection_queue.write().unwrap().push(Vec::new());
            let connection_queue_clone = Arc::clone(&connection_queue);

            thread::spawn(move || {
                let thread_id = id;

                loop {
                    let connection_vec = connection_queue_clone.read().unwrap()[thread_id].clone();
                    connection_queue_clone.write().unwrap()[thread_id].clear();

                    match connection_vec.len() {
                        0 => thread::sleep(Duration::from_millis(100)),
                        _ =>{
                            for update in connection_vec.iter() {
                                sender.send_message(&OwnedMessage::Text(update.to_string())).unwrap()
                            }
                        },
                    };
                }
            });

            println!("This is websocket {}", id);
            id += 1;
        }
    });
}