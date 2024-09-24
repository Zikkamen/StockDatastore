use std::sync::{Arc, RwLock};
use std::collections::{HashSet, HashMap};

use crate::value_store::stock_information_cache::StockInformationCache;
use crate::websockets::notification_server_in::NotificationServerIn;
use crate::websockets::notification_server_out::NotificationServerOut;

pub struct WebSocketServer {
    ip_server_in: String,
    ip_server_out: String,
    stock_list: Vec<String>,
}

impl WebSocketServer {
    pub fn new(ip_server_in: &str, ip_server_out: &str, stock_list: Vec<String>) -> Self {
        WebSocketServer { 
            ip_server_in: ip_server_in.to_string(), 
            ip_server_out: ip_server_out.to_string(),
            stock_list: stock_list,
        }
    }

    pub fn start_server(&self) {
        let connection_queue = Arc::new(RwLock::new(HashMap::<usize, Vec<String>>::new()));
        let stock_information_cache = Arc::new(RwLock::new(StockInformationCache::new()));
        let subscriber_map = Arc::new(RwLock::new(HashMap::<String, HashSet<usize>>::new()));

        for stock_name in self.stock_list.clone().into_iter() {
            stock_information_cache.write().unwrap().add_json(&format!("{{sn:{},si{}}}", stock_name, 1)[..]);
        }

        let notification_server_out = NotificationServerOut::new(
            self.ip_server_out.clone(),
            Arc::clone(&connection_queue), 
            Arc::clone(&subscriber_map), 
            Arc::clone(&stock_information_cache)
        );
        
        notification_server_out.start_server();

        let mut notification_server_in = NotificationServerIn::new(
            self.ip_server_in.clone(),
            Arc::clone(&connection_queue), 
            Arc::clone(&subscriber_map), 
            Arc::clone(&stock_information_cache)
        );

        notification_server_in.start_server();
    }
}