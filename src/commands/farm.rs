use crate::plot::Plot;
use crate::{Tag, PRIME_SIZE_BYTES, SIGNING_CONTEXT};
use jsonrpsee::client::Subscription;
use jsonrpsee::common::Params;
use log::{debug, info, trace};
use schnorrkel::Keypair;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

type SlotNumber = u64;

#[derive(Debug, Serialize)]
struct Solution {
    public_key: [u8; 32],
    nonce: u64,
    encoding: Vec<u8>,
    signature: Vec<u8>,
    tag: Tag,
}

#[derive(Debug, Serialize)]
struct ProposedProofOfSpaceResponse {
    slot_number: SlotNumber,
    solution: Option<Solution>,
}

#[derive(Debug, Deserialize)]
struct SlotInfo {
    slot_number: SlotNumber,
    challenge: [u8; PRIME_SIZE_BYTES],
    solution_range: u64,
}

/// Start farming by using plot in specified path and connecting to WebSocket server at specified
/// address.
pub(crate) async fn farm(path: PathBuf, ws_server: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Connecting to RPC server");
    let client = jsonrpsee::ws_client(ws_server).await?;

    let identity_file = path.join("identity.bin");
    if !identity_file.exists() {
        panic!("Identity not found, please create it first using plot command");
    }

    info!("Opening existing keypair");
    let keypair =
        Keypair::from_bytes(&fs::read(identity_file)?).map_err(|error| error.to_string())?;
    let ctx = schnorrkel::context::signing_context(SIGNING_CONTEXT);

    info!("Opening plot");
    let plot = Plot::open_or_create(&path.into()).await?;

    if plot.is_empty().await {
        panic!("Plot is empty, please create it first using plot command");
    }

    info!("Subscribing to slot info notifications");
    let mut sub: Subscription<SlotInfo> = client
        .subscribe(
            "poc_subscribeSlotInfo",
            Params::None,
            "poc_unsubscribeSlotInfo",
        )
        .await?;

    loop {
        let slot_info = sub.next().await;
        debug!("New slot: {:?}", slot_info);

        let solution = match plot
            .find_by_range(slot_info.challenge, slot_info.solution_range)
            .await?
        {
            Some((tag, index)) => {
                let encoding = plot.read(index).await?;
                let solution = Solution {
                    public_key: keypair.public.to_bytes(),
                    nonce: index,
                    encoding: encoding.to_vec(),
                    signature: keypair.sign(ctx.bytes(&tag)).to_bytes().to_vec(),
                    tag,
                };

                debug!("Solution found");
                trace!("Solution found: {:?}", solution);

                Some(solution)
            }
            None => {
                debug!("Solution not found");
                None
            }
        };

        client
            .request(
                "poc_proposeProofOfSpace",
                Params::Array(vec![serde_json::to_value(&ProposedProofOfSpaceResponse {
                    slot_number: slot_info.slot_number,
                    solution,
                })
                .unwrap()]),
            )
            .await?;
    }
}
