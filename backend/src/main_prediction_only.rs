mod prediction_markets;
mod api_prediction_only;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    
    log::info!("🔮 ====================================================");
    log::info!("🔮 Prediction Markets Trading System");
    log::info!("🔮 ====================================================");
    log::info!("🔮 Focus: Polymarket-style on-chain prediction markets");
    log::info!("🔮 Strategy: Expected Value (EV) based trading");
    log::info!("🔮 Risk Management: Kelly Criterion position sizing");
    log::info!("🔮 ====================================================");
    
    // Start the prediction markets API server
    api_prediction_only::start_prediction_server().await;
}
