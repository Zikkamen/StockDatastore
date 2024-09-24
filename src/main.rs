mod value_store;
mod file_reader;
mod websockets;

use crate::websockets::websocket_server::WebSocketServer;
use crate::file_reader::stock_config_reader::StockConfigReader;

fn main() {
    let stock_list:Vec<String> = StockConfigReader::new().read_config();
    
    let websocket_server = WebSocketServer::new("localhost:9003", "localhost:9004", stock_list);
    websocket_server.start_server();
}
