use std::collections::HashMap;
use std::collections::VecDeque;

pub struct StockInformation {
    pub stock_name: String,
    pub stock_interval: usize,
    pub timestamp:i64,

    pub avg_price: f64,
    pub avg_price_open: f64,
    pub min_price: f64,
    pub max_price: f64,

    pub volume_moved: i64,
    pub num_of_trades: i64,
}

impl StockInformation {
    pub fn new() -> Self {
        StockInformation { 
            stock_name: String::new(),
            stock_interval: 0,
            timestamp: 0,
            avg_price: 0.0,
            avg_price_open: 0.0,
            min_price: 0.0,
            max_price: 0.0,
            volume_moved: 0,
            num_of_trades: 0
        }
    }

    pub fn clone(&self) -> Self {
        StockInformation {
            stock_name: self.stock_name.clone(),
            stock_interval: self.stock_interval,
            timestamp: self.timestamp,
            avg_price: self.avg_price,
            avg_price_open: self.avg_price_open,
            min_price: self.min_price,
            max_price: self.max_price,
            volume_moved: self.volume_moved,
            num_of_trades: self.num_of_trades
        }
    }

    pub fn insert_data(&mut self, key: String, value: String) {
        match key.as_str() {
            "sn" => self.stock_name = value,
            "si" => self.stock_interval = value.parse::<usize>().unwrap(),
            "ap" => self.avg_price = value.parse::<f64>().unwrap(),
            "op" => self.avg_price_open = value.parse::<f64>().unwrap(),
            "mn" => self.min_price = value.parse::<f64>().unwrap(),
            "mx" => self.max_price = value.parse::<f64>().unwrap(),
            "vm" => self.volume_moved = value.parse::<i64>().unwrap(),
            "nt" => self.num_of_trades = value.parse::<i64>().unwrap(),
            "t" => self.timestamp = value.parse::<i64>().unwrap(),
            _ => (),
        }
    }
}

pub struct StockInformationCache {
    stock_info_map: HashMap<String, String>,
    stock_history_map: HashMap<(String, usize), VecDeque<String>>,
}

impl StockInformationCache {
    pub fn new() -> Self {
        StockInformationCache{ stock_info_map:HashMap::new(), stock_history_map:HashMap::new() }
    }

    pub fn add_json(&mut self, json_data: &str) -> String {
        let stock_info:StockInformation = parse_json_to_stock_info(json_data);

        if stock_info.volume_moved != 0 && stock_info.stock_interval == 1 || !self.stock_info_map.contains_key(&stock_info.stock_name) {
            self.stock_info_map.insert(stock_info.stock_name.clone(), json_data.to_string());
        }

        let key:(String, usize) = (stock_info.stock_name.clone(), stock_info.stock_interval);

        if !self.stock_history_map.contains_key(&key) {
            self.stock_history_map.insert(key.clone(), VecDeque::new());
        }

        let stock_history = self.stock_history_map.get_mut(&key).unwrap();

        if stock_history.len() > 120 { 
            stock_history.pop_front(); 
        }

        stock_history.push_back(json_data.to_string());

        key.0
    }

    pub fn get_stock_names(&self) -> String {
        self.stock_history_map.keys()
            .filter(|(stock_name, interval)| *interval == 0)
            .map(|(stock_name, interval)| stock_name)
            .fold(String::new(), |acc, stock| acc + stock + "|")
    }

    pub fn has_key(&self, key: &String) -> bool {
        self.stock_info_map.contains_key(key)
    }

    pub fn get_entire_cache(&self) -> Vec<String> {
        let mut cache_dump = Vec::<String>::new();

        for json_data in self.stock_info_map.values().map(|a| a.to_string()).into_iter() {
            cache_dump.push(json_data);
        }

        for stock_queue in self.stock_history_map.values().into_iter() {
            for json_data in stock_queue.clone().into_iter() {
                cache_dump.push(json_data);
            }
        }

        cache_dump
    }
}

pub fn parse_json_to_stock_info(json_data: &str) -> StockInformation {
    let mut tmp: String = String::new();
    let mut key: String = String::new();
    let mut stock_info = StockInformation::new();

    for p in json_data.chars() {
        if p == ' ' || p == '\n' || p == '\t' || p == '\"' || p == '{' || p == '}' { 
            continue; 
        }
        
        if p == ':' || p == ',' {
            match key.len() {
                0 => key = tmp,
                _ => {
                    stock_info.insert_data(key, tmp);
                    key = String::new();
                }
            }
            
            tmp = String::new();
            
            continue;
        }

        tmp.push(p);
    }

    stock_info.insert_data(key, tmp);

    stock_info
}