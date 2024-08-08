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

use crate::database_clients::postgres_client::PostgresClient;
use crate::value_store::credentials_store::CredentialsStore;

fn main() {
    let connections = Arc::new(RwLock::new(Vec::<Writer<TcpStream>>::new()));
    let connections_clone = Arc::clone(&connections);

    let credentials_store = CredentialsStore::new();
    let mut postgres_client = PostgresClient::new(credentials_store);

    //start_websocketserver(connections_clone);

    println!("{:?}", postgres_client.get_all_tables());

    /*
    loop {
        thread::sleep(Duration::from_millis(1000));

        for sender in connections.write().unwrap().iter_mut() {
            sender.send_message(&OwnedMessage::Text("ping".to_string())).unwrap();
        }
    }
    */
}

fn start_websocketserver(connections: Arc<RwLock<Vec<Writer<TcpStream>>>>){
    let server = Server::bind("127.0.0.1:9002").unwrap();

    thread::spawn(move || {
        for connection in server.filter_map(Result::ok) {
            let client = connection.accept().unwrap();
            let (_receiver, mut sender) = client.split().unwrap();

            sender.send_message(&OwnedMessage::Text("Hello from the server!".to_string())).unwrap();

            connections.write().unwrap().push(sender);
        }
    });
}