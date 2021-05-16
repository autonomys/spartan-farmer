use crate::plot::Plot;
use crate::PRIME_SIZE_BYTES;
use log::{debug, info};
use std::path::PathBuf;
use crate::crypto::hash_challenge;
use rand::Rng;

/// Start farming by using plot in specified path
pub(crate) async fn sim(path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {

    let identity_file = path.join("identity.bin");
    if !identity_file.exists() {
        panic!("Identity not found, please create it first using plot command");
    }

    info!("Opening plot");
    let plot = Plot::open_or_create(&path.into()).await?;

    if plot.is_empty().await {
        panic!("Plot is empty, please create it first using plot command");
    }

    // define initial plot size and solution range
    const EXPECTED_SLOTS_PER_BLOCK: u64 = 6;
    const BLOCKS_PER_ERA: u64 = 2016;
    const SLOTS_PER_ERA: u64 = 2016 * EXPECTED_SLOTS_PER_BLOCK;
    const ERA_TRIALS: u64 = 100;
    const SLOT_TRIALS: u64 = SLOTS_PER_ERA * ERA_TRIALS;
    const INITIAL_PLOT_SIZE: u64 = 1024 * 1024 * 1024 / 4096;
    const INITIAL_SOLUTION_RANGE: u64 = u64::MAX / INITIAL_PLOT_SIZE / EXPECTED_SLOTS_PER_BLOCK;

    let mut challenge = rand::thread_rng().gen::<[u8; PRIME_SIZE_BYTES]>();
    let mut solution_range = INITIAL_SOLUTION_RANGE;
    let mut era_solution_count: u64 = 0;
    let mut era_slot_count: u64 = 0;
    let mut era = 0;

    for i in 0..SLOT_TRIALS {
        debug!("New slot: {:?}", i);
        era_slot_count += 1;

       match plot
            .find_by_range(challenge, solution_range)
            .await?
        {
            Some(_) => {
                debug!("Solution found");
                era_solution_count += 1;
            }
            None => {
                debug!("Solution not found");
            }
        };

        // update the challenge
        challenge = hash_challenge(challenge);

        // check for era boundary
        if era_solution_count > 0 && era_solution_count % BLOCKS_PER_ERA == 0 {
            // update the solution range
            let actual_slots_per_block = if era_solution_count == 0 {
                0f64
            } else {
                (era_slot_count as f64 / era_solution_count as f64) as f64
            };
            let adjustment_factor: f64 = actual_slots_per_block / EXPECTED_SLOTS_PER_BLOCK as f64;
            solution_range = (solution_range as f64 * adjustment_factor).round() as u64;

            era += 1;

            // print results
            info!("Arrived at era transition {}", era);
            info!("Expected slots per block: {}", EXPECTED_SLOTS_PER_BLOCK);
            info!("Actual slots per block: {}", actual_slots_per_block);
            info!("Adjustment factor: {} \n", adjustment_factor);

            era_solution_count = 0;
            era_slot_count = 0;
        }
    }

    Ok(())
}
