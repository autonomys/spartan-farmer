use jsonrpsee::client::Subscription;
use jsonrpsee::common::Params;
use serde::{Deserialize, Serialize};

type SlotNumber = u64;

#[derive(Debug, Serialize)]
pub struct ProposedProofOfSpace {
    slot_number: SlotNumber,
}

#[derive(Debug, Deserialize)]
pub struct SlotInfo {
    slot_number: SlotNumber,
    epoch_randomness: Vec<u8>,
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("Connecting");
    let client = jsonrpsee::ws_client("ws://127.0.0.1:9945").await?;
    println!("Connected");
    let mut sub: Subscription<SlotInfo> = client
        .subscribe(
            "babe_subscribeSlotInfo",
            Params::None,
            "babe_unsubscribeSlotInfo",
        )
        .await?;
    println!("Subscribed");
    loop {
        let slot_info = sub.next().await;
        println!("{:?}", slot_info);

        client
            .notification(
                "babe_proposeProofOfSpace",
                Params::Array(vec![serde_json::to_value(&ProposedProofOfSpace {
                    slot_number: slot_info.slot_number,
                })
                .unwrap()]),
            )
            .await;
    }
}
