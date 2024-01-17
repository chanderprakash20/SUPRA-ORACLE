use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::time;
use ed25519_dalek::{Keypair, Signer, Verifier};
use rand::rngs::OsRng;
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheResult {
average_price: f64,
data_points: Vec<f64>,
signature: Vec<u8>,
}
async fn cache_mode(
times: usize,
sender: tokio::sync::mpsc::Sender<CacheResult>,
keypair: Arc<Keypair>,
) {
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
signature: keypair.sign(&average_price.to_be_bytes()).to_bytes().to_vec(),
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
let public_keys: Vec<_> = results
.iter()
.map(|result| {
let public_key = ed25519_dalek::PublicKey::from_bytes(&result.signature[32..])
.expect("Failed to deserialize public key");
(result.average_price, public_key)
})
.collect();
for result in results {
let public_key = ed25519_dalek::PublicKey::from_bytes(&result.signature[32..])
.expect("Failed to deserialize public key");
let signature = ed25519_dalek::Signature::from_bytes(&result.signature[..32])
.expect("Failed to deserialize signature");
if public_key.verify(&result.average_price.to_be_bytes(), &signature).is_err() {
panic!("Invalid signature");
}
}
let total_average_price = public_keys
.iter()
.map(|(price, _)| price)
.sum::<f64>()
/ public_keys.len() as f64;
println!(
"Aggregator complete. The total average USD price of BTC is: {}",
total_average_price
);
}
fn generate_keypair() -> Keypair {
let mut csprng = OsRng;
Keypair::generate(&mut csprng)
}
#[tokio::main]
async fn main() {
let num_clients = 5;
let tick_duration = Duration::from_secs(10);
let (sender, receiver) = tokio::sync::mpsc::channel::<CacheResult>(num_clients);
let aggregator_keypair = generate_keypair();
let aggregator_keypair = Arc::new(aggregator_keypair);
let aggregator_handle = tokio::spawn(aggregator(receiver, num_clients));
let mut handles = Vec::with_capacity(num_clients);
for _ in 0..num_clients {
let sender_clone = sender.clone();
let keypair = Arc::new(generate_keypair());
let handle = tokio::spawn(async move {
cache_mode(10, sender_clone, keypair).await;
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
}