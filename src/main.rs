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
    tag: [u8; 32],
    randomness: Vec<u8>,
}

#[derive(Debug, Serialize)]
pub struct ProposedProofOfSpaceResponse {
    slot_number: SlotNumber,
    solution: Option<Solution>,
    tag: [u8; 32],
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
    let client = jsonrpsee::ws_client("ws://127.0.0.1:9944").await?;
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
        let result = client
            .request(
                "babe_proposeProofOfSpace",
                Params::Array(vec![serde_json::to_value(&ProposedProofOfSpaceResponse {
                    slot_number: slot_info.slot_number,
                    solution: Some(Solution {
                        public_key: [0u8; 32],
                        nonce: 0,
                        encoding: vec![],
                        signature: [0u8; 32],
                        tag: [0u8; 32],
                        randomness: vec![],
                    }),
                    tag: [0u8; 32],
                })
                .unwrap()]),
            )
            .await;
        match result {
            Ok(()) => {
                print!("Solution sent successfully");
            }
            Err(error) => {
                eprint!("Failed to send solution: {:?}", error);
            }
        }
    }
}
