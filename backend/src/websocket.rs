use futures::{SinkExt, StreamExt};
use tokio::sync::broadcast;
use warp::ws::{Message, WebSocket};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketUpdate {
    pub symbol: String,
    pub price: f64,
    pub volume: f64,
    pub timestamp: i64,
    pub change_24h: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeUpdate {
    pub id: String,
    pub symbol: String,
    pub action: String,
    pub price: f64,
    pub size: f64,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WSMessage {
    MarketUpdate(MarketUpdate),
    TradeUpdate(TradeUpdate),
    PortfolioUpdate {
        total_value: f64,
        cash: f64,
        positions: Vec<(String, f64)>,
    },
    Error {
        message: String,
    },
    Ping,
    Pong,
}

pub type WSBroadcaster = broadcast::Sender<WSMessage>;

/// Handle WebSocket connection
pub async fn handle_websocket(ws: WebSocket, broadcaster: WSBroadcaster) {
    let (mut ws_tx, mut ws_rx) = ws.split();
    let mut rx = broadcaster.subscribe();

    log::info!("New WebSocket connection established");

    // Send initial ping
    if let Err(e) = ws_tx.send(Message::text(
        serde_json::to_string(&WSMessage::Ping).unwrap()
    )).await {
        log::error!("Failed to send ping: {}", e);
        return;
    }

    // Spawn task to forward broadcast messages to client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            match serde_json::to_string(&msg) {
                Ok(json) => {
                    if ws_tx.send(Message::text(json)).await.is_err() {
                        log::warn!("WebSocket send failed, closing connection");
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Failed to serialize message: {}", e);
                }
            }
        }
    });

    // Handle incoming messages from client
    while let Some(result) = ws_rx.next().await {
        match result {
            Ok(msg) => {
                if msg.is_text() {
                    if let Ok(text) = msg.to_str() {
                        log::debug!("Received WebSocket message: {}", text);
                        // Handle pong or other client messages
                        if let Ok(parsed) = serde_json::from_str::<WSMessage>(text) {
                            match parsed {
                                WSMessage::Ping => {
                                    // Respond with pong (handled by broadcaster)
                                }
                                _ => {
                                    log::debug!("Received message: {:?}", parsed);
                                }
                            }
                        }
                    }
                } else if msg.is_close() {
                    log::info!("WebSocket connection closed by client");
                    break;
                }
            }
            Err(e) => {
                log::error!("WebSocket error: {}", e);
                break;
            }
        }
    }

    // Clean up
    send_task.abort();
    log::info!("WebSocket connection terminated");
}

/// Create a broadcast channel for WebSocket messages
pub fn create_ws_broadcaster() -> WSBroadcaster {
    let (tx, _) = broadcast::channel(100);
    tx
}

/// Broadcast market update to all connected clients
pub fn broadcast_market_update(
    broadcaster: &WSBroadcaster,
    symbol: String,
    price: f64,
    volume: f64,
    change_24h: f64,
) {
    let update = WSMessage::MarketUpdate(MarketUpdate {
        symbol,
        price,
        volume,
        timestamp: chrono::Utc::now().timestamp(),
        change_24h,
    });

    if let Err(e) = broadcaster.send(update) {
        log::warn!("Failed to broadcast market update: {}", e);
    }
}

/// Broadcast trade update to all connected clients
pub fn broadcast_trade_update(
    broadcaster: &WSBroadcaster,
    id: String,
    symbol: String,
    action: String,
    price: f64,
    size: f64,
) {
    let update = WSMessage::TradeUpdate(TradeUpdate {
        id,
        symbol,
        action,
        price,
        size,
        timestamp: chrono::Utc::now().timestamp(),
    });

    if let Err(e) = broadcaster.send(update) {
        log::warn!("Failed to broadcast trade update: {}", e);
    }
}
