use std::{fs::File, io::Write};
use serde::{Deserialize, Serialize};
use websocket::{ClientBuilder, Message};
#[derive(Debug, Serialize, Deserialize)]
struct CacheResult {
average_price: f64,
data_points: Vec<f64>,
}
async fn cache_mode(times: usize, output_file: &str) {
let url = "wss://stream.binance.com:9443/ws/btcusdt@trade"; // Binance WebSocket URL for
BTC/USDT trades
let mut prices = Vec::new();
let mut websocket = ClientBuilder::new(url).wait_for_message().await.expect("Failed to
connect to WebSocket");
for i in 0..times {
let reply = websocket.recv().await.expect("Failed to receive data from WebSocket");
let datum: serde_json::Value = serde_json::from_str(&reply.to_text().unwrap()).unwrap();
if let (Some(symbol), Some(price)) = (datum.get("s"), datum.get("p")) {
if symbol == "BTCUSDT" {
prices.push(price.as_str().unwrap().parse::<f64>().unwrap());
}
}
}
let average_price = prices.iter().sum::<f64>() / prices.len() as f64;
println!("Cache complete. The average USD price of BTC is: {}", average_price);
let result = CacheResult {average_price,data_points : prices};
const OUTPUT_FILE_NAME :&'static str=output_file;
let mut file = File::create(output_file).expect("Failed to create output file");
file.write_all(serde_json::to_string(&result).unwrap().as_bytes())
.expect("Failed to write to output file");
}
async fn read_mode(input_file: &str) {
let file = File::open(input_file).expect("Failed to open input file");
let data: CacheResult = serde_json::from_reader(file).expect("Failed to parse input file");
println!("Average USD price of BTC: {}", data.average_price);
println!("Data Points: {:?}", data.data_points);
}
#[tokio::main]
async fn main() {
let args: Vec<String> = std::env::args().collect();
match args.get(1).map(|s| s.as_str()) {
Some("--mode=cache") => {
let times = args.get(2).unwrap_or(&"10".to_string()).parse().unwrap();
let output_file = args.get(3).unwrap_or(&"output.json".to_string());
cache_mode(times, output_file).await;
}
Some("--mode=read") => {
let input_file = args.get(2).unwrap_or(&"output.json".to_string());
read_mode(input_file).await;
}
_ => {
println!("Usage: ./simple --mode=cache --times=10");
println!(" ./simple --mode=read");
}
}
}
