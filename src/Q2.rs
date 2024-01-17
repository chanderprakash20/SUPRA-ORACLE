use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time;
#[derive(Debug, Clone)]
struct CacheResult {
average_price: f64,
data_points: Vec<f64>,
}
async fn cache_mode(times: usize, sender: tokio::sync::mpsc::Sender<CacheResult>) {
let url = "wss://stream.binance.com:9443/ws/btcusdt@trade"; // Binance WebSocket URL for
BTC/USDT trades
let mut prices = Vec::new();
let mut websocket = websocket::ClientBuilder::new(url)
.async_connect(None, None)
.await
.expect("Failed to connect to WebSocket");
for _ in 0..times {
let response = websocket.recv().await.expect("Failed to receive data from WebSocket");
let data: serde_json::Value = serde_json::from_str(&response.to_text().unwrap()).unwrap();
if let (Some(symbol), Some(price)) = (data.get("s"), data.get("p")) {
if symbol == "BTCUSDT" {
prices.push(price.as_str().unwrap().parse::<f64>().unwrap());
}
}
}
let average_price = prices.iter().sum::<f64>() / prices.len() as f64;
let result = CacheResult {
average_price,
data_points: prices,
};
sender.send(result).await.expect("Failed to send data to aggregator");
}
async fn aggregator(receiver: tokio::sync::mpsc::Receiver<CacheResult>, num_clients: usize) {
let mut results = Vec::with_capacity(num_clients);
for _ in 0..num_clients {
if let Some(result) = receiver.recv().await {
results.push(result);
}
}
let total_average_price = results
.iter()
.map(|result| result.average_price)
.sum::<f64>()
/ results.len() as f64;
println!("Aggregator complete. The total average USD price of BTC is: {}",
total_average_price);
}
#[tokio::main]
async fn main() {
let num_clients = 5;
let tick_duration = Duration::from_secs(10);
let (sender, receiver) = tokio::sync::mpsc::channel::<CacheResult>(num_clients);
let aggregator_handle = tokio::spawn(aggregator(receiver, num_clients));
let mut handles = Vec::with_capacity(num_clients);
for _ in 0..num_clients {
let sender_clone = sender.clone();
let handle = tokio::spawn(async move {
cache_mode(10, sender_clone).await;
});
handles.push(handle);
}
// Wait for all client processes to complete
for handle in handles {
handle.await.expect("Error in client process");
}
// Drop sender to signal the aggregator process that all data has been sent
drop(sender);
// Wait for the aggregator process to complete
aggregator_handle.await.expect("Error in aggregator process");