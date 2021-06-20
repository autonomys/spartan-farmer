use crate::plot::Plot;
use crate::{crypto, Salt, Tag, PRIME_SIZE_BYTES, SIGNING_CONTEXT};
use async_std::task;
use futures::channel::oneshot;
use jsonrpsee::ws_client::traits::{Client, SubscriptionClient};
use jsonrpsee::ws_client::v2::params::JsonRpcParams;
use jsonrpsee::ws_client::{Subscription, WsClientBuilder};
use log::{debug, error, info, trace, warn};
use ring::digest;
use schnorrkel::Keypair;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

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

/// Information about new slot that just arrived
#[derive(Debug, Deserialize)]
struct SlotInfo {
    /// Slot number
    slot_number: SlotNumber,
    /// Slot challenge
    challenge: [u8; PRIME_SIZE_BYTES],
    /// Salt
    salt: Salt,
    /// Salt for the next eon
    next_salt: Option<Salt>,
    /// Acceptable solution range
    solution_range: u64,
}

/// Start farming by using plot in specified path and connecting to WebSocket server at specified
/// address.
pub(crate) async fn farm(path: PathBuf, ws_server: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Connecting to RPC server");
    let client = WsClientBuilder::default().build(&ws_server).await?;

    let identity_file = path.join("identity.bin");
    if !identity_file.exists() {
        panic!("Identity not found, please create it first using plot command");
    }

    info!("Opening existing keypair");
    let keypair =
        Keypair::from_bytes(&fs::read(identity_file)?).map_err(|error| error.to_string())?;
    let public_key_hash = crypto::hash_public_key(&keypair.public);
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
            JsonRpcParams::NoParams,
            "poc_unsubscribeSlotInfo",
        )
        .await?;

    // TODO: Add cleanup mechanism for old commitments after restart
    let mut current_salt = None;
    let mut next_salt = None;

    while let Some(slot_info) = sub.next().await {
        debug!("New slot: {:?}", slot_info);

        if current_salt != Some(slot_info.salt) {
            if next_salt == Some(slot_info.salt) {
                let old_salt = current_salt.replace(slot_info.salt);
                if let Some(old_salt) = old_salt {
                    info!("Salt is out of date, removing commitment");

                    task::spawn({
                        let plot = plot.clone();

                        async move {
                            if let Err(error) = plot.remove_commitment(old_salt).await {
                                error!("Failed to remove old commitment: {}", error);
                            }
                        }
                    })
                    .await;
                }
            } else {
                let started = Instant::now();
                info!("Salt update, recommitting");
                if let Err(error) = plot.create_commitment(slot_info.salt).await {
                    error!("Failed to create commitment: {}", error);
                    continue;
                }
                let old_salt = current_salt.replace(slot_info.salt);
                if let Some(old_salt) = old_salt {
                    warn!("New salt is not the same as previously known next salt");
                    info!("Salt is out of date, removing commitment");

                    task::spawn({
                        let plot = plot.clone();

                        async move {
                            if let Err(error) = plot.remove_commitment(old_salt).await {
                                error!("Failed to remove old commitment: {}", error);
                            }
                        }
                    })
                    .await;
                }
                info!(
                    "Finished recommitment in {} seconds",
                    started.elapsed().as_secs_f32()
                );
            }
        }
        if let Some(new_next_salt) = slot_info.next_salt {
            if Some(new_next_salt) != next_salt {
                let old_salt = next_salt.replace(new_next_salt);
                if old_salt != current_salt {
                    if let Some(old_salt) = old_salt {
                        warn!("Previous next salt is out of date, removing commitment");

                        task::spawn({
                            let plot = plot.clone();

                            async move {
                                if let Err(error) = plot.remove_commitment(old_salt).await {
                                    error!("Failed to remove old commitment: {}", error);
                                }
                            }
                        })
                        .await;
                    }
                }

                task::spawn({
                    let plot = plot.clone();

                    async move {
                        let started = Instant::now();
                        info!("Salt will update soon, recommitting in background");
                        if let Err(error) = plot.create_commitment(new_next_salt).await {
                            error!("Recommitting salt in background failed: {}", error);
                            return;
                        }
                        info!(
                            "Finished recommitment in background in {} seconds",
                            started.elapsed().as_secs_f32()
                        );
                    }
                });
            }
        }

        let local_challenge = derive_local_challenge(&slot_info.challenge, &public_key_hash);

        let solution = match plot
            .find_by_range(local_challenge, slot_info.solution_range, slot_info.salt)
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
                JsonRpcParams::Array(vec![serde_json::to_value(&ProposedProofOfSpaceResponse {
                    slot_number: slot_info.slot_number,
                    solution,
                })
                .unwrap()]),
            )
            .await?;
    }

    let (tx, rx) = oneshot::channel();

    let _handler = plot.on_close(move || {
        let _ = tx.send(());
    });

    drop(plot);

    rx.await?;

    Ok(())
}

fn derive_local_challenge(global_challenge: &[u8], farmer_id: &[u8]) -> [u8; 8] {
    digest::digest(&digest::SHA256, &{
        let mut data = Vec::with_capacity(global_challenge.len() + farmer_id.len());
        data.extend_from_slice(global_challenge);
        data.extend_from_slice(farmer_id);
        data
    })
    .as_ref()[..8]
        .try_into()
        .unwrap()
}
