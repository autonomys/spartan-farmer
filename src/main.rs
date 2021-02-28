use jsonrpsee::client::Subscription;
use jsonrpsee::common::Params;
use serde::{Deserialize, Serialize};

type SlotNumber = u64;

#[derive(Debug, Serialize)]
struct Solution {
    public_key: [u8; 32],
    nonce: u32,
    encoding: Vec<u8>,
    signature: [u8; 32],
}

#[derive(Debug, Serialize)]
pub struct ProposedProofOfSpaceResponse {
    slot_number: SlotNumber,
    solution: Option<Solution>,
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
        // TODO: Evaluate plot
        let solution = None;
        client
            .notification(
                "babe_proposeProofOfSpace",
                Params::Array(vec![serde_json::to_value(&ProposedProofOfSpaceResponse {
                    slot_number: slot_info.slot_number,
                    solution,
                })
                .unwrap()]),
            )
            .await;
    }
}
