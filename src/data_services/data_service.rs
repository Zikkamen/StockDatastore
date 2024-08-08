use crate::database_clients::postgres_client::PostgresClient;
use crate::value_store::credentials_store::CredentialsStore;

#[derive(Debug, Clone)]
pub struct StockData {
    pub name: String,
    pub time: i64,
    pub num_of_trades: i32,
    pub volume_moved: i32,
    pub avg_price: f64,
    pub min_price: f64,
    pub max_price: f64,
}

pub struct DataService {
    stock_data: Vec<StockData>,
    list_of_stocks: Vec<String>,
    postgres_client: PostgresClient,
}

impl DataService {
    pub fn new(credentials_store: CredentialsStore) -> Self {
        let mut data_service = DataService {
            stock_data: Vec::new(),
            list_of_stocks: Vec::new(),
            postgres_client: PostgresClient::new(credentials_store),
        };

        data_service.init();

        data_service
    }

    fn init(&mut self) {
        self.list_of_stocks = self.postgres_client.get_all_tables();
        self.stock_data = self.get_list_of_current_data();
    }

    pub fn get_stock_data_copy(&self) -> Vec<StockData> {
        self.stock_data.clone()
    }

    pub fn check_for_updates(&mut self) -> Vec<StockData> {
        let current_list = self.get_list_of_current_data();
        let mut changed_list = Vec::new();

        let n = current_list.len();

        for i in 0..n {
            if self.stock_data[i].time != current_list[i].time {
                changed_list.push(current_list[i].clone());

                self.stock_data[i] = current_list[i].clone();
            }
        }

        changed_list
    }

    fn get_list_of_current_data(&mut self) -> Vec<StockData> {
        let mut stock_data_list = Vec::new();

        for stock in self.list_of_stocks.iter() {
            let mut row = self.postgres_client.get_most_recent_data(stock);

            match row {
                Some(p) =>{
                    let data = StockData {
                        name: stock.to_string(),
                        time: p.get("time"),
                        num_of_trades: p.get("num_of_trades"),
                        volume_moved: p.get("volume_moved"),
                        avg_price: p.get::<&str, i64>("avg_price") as f64 / 100.0,
                        min_price: p.get::<&str, i64>("min_price") as f64 / 100.0,
                        max_price: p.get::<&str, i64>("max_price") as f64 / 100.0,
                    };

                    stock_data_list.push(data);
                },
                None => {
                    let data = StockData {
                        name: stock.to_string(),
                        time: 0,
                        num_of_trades: 0,
                        volume_moved: 0,
                        avg_price: 0.0,
                        min_price: 0.0,
                        max_price: 0.0,
                    };

                    stock_data_list.push(data);
                },
            }
        }

        stock_data_list
    }
}