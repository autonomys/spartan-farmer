use jsonrpsee::common::Params;
use serde::Deserialize;
use jsonrpsee::client::Subscription;

#[derive(Debug, Deserialize)]
pub struct SlotInfo {
    pub slot_number: u64,
    pub epoch_randomness: Vec<u8>,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Connecting");
    let client = jsonrpsee::ws_client("ws://127.0.0.1:9944").await?;
    println!("Connected");
    let mut sub: Subscription<SlotInfo> = client.subscribe("babe_subscribeSlotInfo", Params::None, "babe_unsubscribeSlotInfo").await?;
    println!("Subscribed");
    while let message = sub.next().await {
        println!("{:?}", message);
        client.request("babe_proposeProofOfSpace", Params::None).await?;
    }

    Ok(())
}
