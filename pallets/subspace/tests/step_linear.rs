mod mock;

use frame_support::assert_ok;
use log::info;
use mock::*;
use pallet_subspace::{
    DaoTreasuryDistribution, GlobalDaoTreasury, MaxAllowedWeights, MinAllowedWeights, MinBurn,
    SubnetStakeThreshold, Tempo, Trust,
};
use sp_core::U256;
use sp_runtime::Percent;

fn update_params(netuid: u16, tempo: u16, max_weights: u16, min_weights: u16) {
    Tempo::<Test>::insert(netuid, tempo);
    MaxAllowedWeights::<Test>::insert(netuid, max_weights);
    MinAllowedWeights::<Test>::insert(netuid, min_weights);
}

fn check_network_stats(netuid: u16) {
    let emission_buffer: u64 = 1_000; // the numbers arent perfect but we want to make sure they fall within a range (10_000 / 2**64)
    let threshold = SubspaceModule::get_subnet_stake_threshold();
    let subnet_emission: u64 = SubspaceModule::calculate_network_emission(netuid, threshold);
    let incentives: Vec<u16> = SubspaceModule::get_incentives(netuid);
    let dividends: Vec<u16> = SubspaceModule::get_dividends(netuid);
    let emissions: Vec<u64> = SubspaceModule::get_emissions(netuid);
    let total_incentives: u16 = incentives.iter().sum();
    let total_dividends: u16 = dividends.iter().sum();
    let total_emissions: u64 = emissions.iter().sum();

    info!("total_emissions: {total_emissions}");
    info!("total_incentives: {total_incentives}");
    info!("total_dividends: {total_dividends}");

    info!("emission: {emissions:?}");
    info!("incentives: {incentives:?}");
    info!("dividends: {dividends:?}");

    assert!(
        total_emissions >= subnet_emission - emission_buffer
            || total_emissions <= subnet_emission + emission_buffer
    );
}

#[test]
fn test_stale_weights() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;
        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        register_n_modules(0, 10, 1000);
        let _subnet_params = SubspaceModule::subnet_params(netuid);
        let _keys = SubspaceModule::get_keys(netuid);
        let _uids = SubspaceModule::get_uids(netuid);
    });
}

#[test]
fn test_no_weights() {
    new_test_ext().execute_with(|| {
        let netuid: u16 = 0;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        register_n_modules(0, 10, 1000);
        Tempo::<Test>::insert(netuid, 1);
        let _keys = SubspaceModule::get_keys(netuid);
        let _uids = SubspaceModule::get_uids(netuid);

        let incentives: Vec<u16> = SubspaceModule::get_incentives(netuid);
        let dividends: Vec<u16> = SubspaceModule::get_dividends(netuid);
        let emissions: Vec<u64> = SubspaceModule::get_emissions(netuid);
        let _total_incentives: u16 = incentives.iter().sum();
        let _total_dividends: u16 = dividends.iter().sum();
        let _total_emissions: u64 = emissions.iter().sum();
    });
}

#[test]
fn test_dividends_same_stake() {
    new_test_ext().execute_with(|| {
        // CONSSTANTS
        let netuid: u16 = 0;
        let n: u16 = 10;
        let _n_list: Vec<u16> = vec![10, 50, 100, 1000];
        let _blocks_per_epoch_list: u64 = 1;
        let stake_per_module: u64 = 10_000;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        // SETUP NETWORK
        register_n_modules(netuid, n, stake_per_module);
        update_params(netuid, 1, n, 0);

        let keys = SubspaceModule::get_keys(netuid);
        let _uids = SubspaceModule::get_uids(netuid);

        // do a list of ones for weights
        let weight_uids: Vec<u16> = [2, 3].to_vec();
        // do a list of ones for weights
        let weight_values: Vec<u16> = [2, 1].to_vec();
        set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
        set_weights(netuid, keys[1], weight_uids.clone(), weight_values.clone());

        let stakes_before: Vec<u64> = get_stakes(netuid);
        step_epoch(netuid);
        let incentives: Vec<u16> = SubspaceModule::get_incentives(netuid);
        let dividends: Vec<u16> = SubspaceModule::get_dividends(netuid);
        let emissions: Vec<u64> = SubspaceModule::get_emissions(netuid);
        let stakes: Vec<u64> = get_stakes(netuid);

        // evaluate votees
        assert!(incentives[2] > 0);
        assert_eq!(dividends[2], dividends[3]);
        let delta: u64 = 100;
        assert!((incentives[2] as u64) > (weight_values[0] as u64 * incentives[3] as u64) - delta);
        assert!((incentives[2] as u64) < (weight_values[0] as u64 * incentives[3] as u64) + delta);

        assert!(emissions[2] > (weight_values[0] as u64 * emissions[3]) - delta);
        assert!(emissions[2] < (weight_values[0] as u64 * emissions[3]) + delta);

        // evaluate voters
        assert!(
            dividends[0] == dividends[1],
            "dividends[0]: {} != dividends[1]: {}",
            dividends[0],
            dividends[1]
        );
        assert!(
            dividends[0] == dividends[1],
            "dividends[0]: {} != dividends[1]: {}",
            dividends[0],
            dividends[1]
        );

        assert_eq!(incentives[0], incentives[1]);
        assert_eq!(dividends[2], dividends[3]);

        info!("emissions: {emissions:?}");

        for (uid, emission) in emissions.iter().enumerate() {
            if emission == &0 {
                continue;
            }
            let stake: u64 = stakes[uid];
            let stake_before: u64 = stakes_before[uid];
            let stake_difference: u64 = stake - stake_before;
            let expected_stake_difference: u64 = emissions[uid];
            let error_delta: u64 = (emissions[uid] as f64 * 0.001) as u64;

            assert!(
                stake_difference < expected_stake_difference + error_delta
                    && stake_difference > expected_stake_difference - error_delta,
                "stake_difference: {} != expected_stake_difference: {}",
                stake_difference,
                expected_stake_difference
            );
        }

        check_network_stats(netuid);
    });
}

#[test]
fn test_dividends_diff_stake() {
    new_test_ext().execute_with(|| {
        // CONSSTANTS
        let netuid: u16 = 0;
        let n: u16 = 10;
        let _n_list: Vec<u16> = vec![10, 50, 100, 1000];
        let _blocks_per_epoch_list: u64 = 1;
        let stake_per_module: u64 = 10_000;
        let tempo: u16 = 100;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        // SETUP NETWORK
        for i in 0..n {
            let mut stake = stake_per_module;
            if i == 0 {
                stake = 2 * stake_per_module
            }
            let key: U256 = U256::from(i);
            assert_ok!(register_module(netuid, key, stake));
        }
        update_params(netuid, tempo, n, 0);

        let keys = SubspaceModule::get_keys(netuid);
        let _uids = SubspaceModule::get_uids(netuid);

        // do a list of ones for weights
        let weight_uids: Vec<u16> = [2, 3].to_vec();
        // do a list of ones for weights
        let weight_values: Vec<u16> = [1, 1].to_vec();
        set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
        set_weights(netuid, keys[1], weight_uids.clone(), weight_values.clone());

        let stakes_before: Vec<u64> = get_stakes(netuid);
        step_epoch(netuid);
        let incentives: Vec<u16> = SubspaceModule::get_incentives(netuid);
        let dividends: Vec<u16> = SubspaceModule::get_dividends(netuid);
        let emissions: Vec<u64> = SubspaceModule::get_emissions(netuid);
        let stakes: Vec<u64> = get_stakes(netuid);

        // evaluate votees
        assert!(incentives[2] > 0);
        assert_eq!(dividends[2], dividends[3]);
        let delta: u64 = 100;
        assert!((incentives[2] as u64) > (weight_values[0] as u64 * incentives[3] as u64) - delta);
        assert!((incentives[2] as u64) < (weight_values[0] as u64 * incentives[3] as u64) + delta);

        assert!(emissions[2] > (weight_values[0] as u64 * emissions[3]) - delta);
        assert!(emissions[2] < (weight_values[0] as u64 * emissions[3]) + delta);

        // evaluate voters
        let delta: u64 = 100;
        assert!((dividends[0] as u64) > (dividends[1] as u64 * 2) - delta);
        assert!((dividends[0] as u64) < (dividends[1] as u64 * 2) + delta);

        assert_eq!(incentives[0], incentives[1]);
        assert_eq!(dividends[2], dividends[3]);

        info!("emissions: {emissions:?}");

        for (uid, emission) in emissions.iter().enumerate() {
            if emission == &0 {
                continue;
            }
            let stake: u64 = stakes[uid];
            let stake_before: u64 = stakes_before[uid];
            let stake_difference: u64 = stake - stake_before;
            let expected_stake_difference: u64 = emissions[uid];
            let error_delta: u64 = (emissions[uid] as f64 * 0.001) as u64;

            assert!(
                stake_difference < expected_stake_difference + error_delta
                    && stake_difference > expected_stake_difference - error_delta,
                "stake_difference: {} != expected_stake_difference: {}",
                stake_difference,
                expected_stake_difference
            );
        }
        check_network_stats(netuid);
    });
}

#[test]
fn test_pruning() {
    new_test_ext().execute_with(|| {
        // CONSTANTS
        let netuid: u16 = 0;
        let n: u16 = 100;
        let stake_per_module: u64 = 10_000;
        let tempo: u16 = 100;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);
        SubspaceModule::set_max_registrations_per_block(1000);

        // SETUP NETWORK
        register_n_modules(netuid, n, stake_per_module);
        SubspaceModule::set_max_allowed_modules(n);
        update_params(netuid, 1, n, 0);

        let voter_idx = 0;
        let keys = SubspaceModule::get_keys(netuid);
        let _uids = SubspaceModule::get_uids(netuid);

        // Create a list of UIDs excluding the voter_idx
        let weight_uids: Vec<u16> = (0..n).filter(|&x| x != voter_idx as u16).collect();

        // Create a list of ones for weights, excluding the voter_idx
        let mut weight_values: Vec<u16> = weight_uids.iter().map(|_x| 1_u16).collect();

        let prune_uid: u16 = weight_uids.last().cloned().unwrap_or(0);

        if let Some(prune_idx) = weight_uids.iter().position(|&uid| uid == prune_uid) {
            weight_values[prune_idx] = 0;
        }

        set_weights(
            netuid,
            keys[voter_idx as usize],
            weight_uids.clone(),
            weight_values.clone(),
        );

        step_block(tempo);

        let lowest_priority_uid: u16 = SubspaceModule::get_lowest_uid(netuid, false).unwrap_or(0);
        assert!(lowest_priority_uid == prune_uid);

        let new_key: U256 = U256::from(n + 1);

        assert_ok!(register_module(netuid, new_key, stake_per_module));

        let is_registered: bool = SubspaceModule::key_registered(netuid, &new_key);
        assert!(is_registered);

        assert!(
            SubspaceModule::get_subnet_n(netuid) == n,
            "SubspaceModule::get_subnet_n(netuid): {} != n: {}",
            SubspaceModule::get_subnet_n(netuid),
            n
        );

        let is_prune_registered: bool =
            SubspaceModule::key_registered(netuid, &keys[prune_uid as usize]);
        assert!(!is_prune_registered);

        check_network_stats(netuid);
    });
}

#[test]
fn test_lowest_priority_mechanism() {
    new_test_ext().execute_with(|| {
        // CONSSTANTS
        let netuid: u16 = 0;
        let n: u16 = 100;
        let stake_per_module: u64 = 10_000;
        let tempo: u16 = 100;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);
        SubspaceModule::set_max_registrations_per_block(1000);
        // SETUP NETWORK
        register_n_modules(netuid, n, stake_per_module);

        update_params(netuid, tempo, n, 0);

        let keys = SubspaceModule::get_keys(netuid);
        let voter_idx = 0;

        // Create a list of UIDs excluding the voter_idx
        let weight_uids: Vec<u16> = (0..n).filter(|&x| x != voter_idx).collect();

        // Create a list of ones for weights, excluding the voter_idx
        let mut weight_values: Vec<u16> = weight_uids.iter().map(|_x| 1_u16).collect();

        let prune_uid: u16 = n - 1;

        // Check if the prune_uid is still valid after excluding the voter_idx
        if prune_uid != voter_idx {
            // Find the index of prune_uid in the updated weight_uids vector
            if let Some(prune_idx) = weight_uids.iter().position(|&uid| uid == prune_uid) {
                weight_values[prune_idx] = 0;
            }
        }

        set_weights(
            netuid,
            keys[voter_idx as usize],
            weight_uids.clone(),
            weight_values.clone(),
        );
        step_block(tempo);
        let incentives: Vec<u16> = SubspaceModule::get_incentives(netuid);
        let dividends: Vec<u16> = SubspaceModule::get_dividends(netuid);
        let emissions: Vec<u64> = SubspaceModule::get_emissions(netuid);
        let _stakes: Vec<u64> = get_stakes(netuid);

        assert!(emissions[prune_uid as usize] == 0);
        assert!(incentives[prune_uid as usize] == 0);
        assert!(dividends[prune_uid as usize] == 0);

        let lowest_priority_uid: u16 = SubspaceModule::get_lowest_uid(netuid, false).unwrap_or(0);
        info!("lowest_priority_uid: {lowest_priority_uid}");
        info!("prune_uid: {prune_uid}");
        info!("emissions: {emissions:?}");
        info!("lowest_priority_uid: {lowest_priority_uid:?}");
        info!("dividends: {dividends:?}");
        info!("incentives: {incentives:?}");
        assert!(lowest_priority_uid == prune_uid);
        check_network_stats(netuid);
    });
}

// #[test]
// fn test_deregister_zero_emission_uids() {
// 	new_test_ext().execute_with(|| {
//     // CONSSTANTS
//     let netuid: u16 = 0;
//     let n : u16 = 100;
//     let num_zero_uids : u16 = 10;
//     let blocks_per_epoch_list : u64 = 1;
//     let stake_per_module : u64 = 10_000;

//     // SETUP NETWORK
//     let tempo: u16 = 1;
//     register_n_modules( netuid, n, stake_per_module );
//     SubspaceModule::set_tempo( netuid, tempo );
//     SubspaceModule::set_max_allowed_weights(netuid, n );
//     SubspaceModule::set_min_allowed_weights(netuid, 0 );
//     SubspaceModule::set_immunity_period(netuid, tempo );

//     let keys = SubspaceModule::get_keys( netuid );
//     let uids = SubspaceModule::get_uids( netuid );
//     // do a list of ones for weights
//     let weight_uids : Vec<u16> = (0..n).collect();
//     // do a list of ones for weights
//     let mut weight_values : Vec<u16> = weight_uids.iter().map(|x| 1 as u16 ).collect();

//     let mut shuffled_uids: Vec<u16> = weight_uids.clone().to_vec();
//     shuffled_uids.shuffle(&mut thread_rng());

//     let mut zero_uids : Vec<u16> = shuffled_uids[0..num_zero_uids as usize].to_vec();

//     for uid in zero_uids.iter() {
//         weight_values[*uid as usize] = 0;

//     }
//     let old_n  : u16 = SubspaceModule::get_subnet_n( netuid );
//     set_weights(netuid, keys[0], weight_uids.clone() , weight_values.clone() );
//     step_block( tempo );
//     let n: u16 = SubspaceModule::get_subnet_n( netuid );
//     assert !( old_n - num_zero_uids == n );

//     });

// }

// TODO:
// #[test]
// fn test_with_weights() {
// 	new_test_ext().execute_with(|| {
// 		let n_list: Vec<u16> = vec![10, 50, 100, 1000];
// 		let blocks_per_epoch_list: u64 = 1;
// 		let stake_per_module: u64 = 10_000;

// 		for (netuid, n) in n_list.iter().enumerate() {
// 			info!("netuid: {}", netuid);
// 			let netuid: u16 = netuid as u16;
// 			let n: u16 = *n;

// 			for i in 0..n {
// 				info!("i: {}", i);
// 				info!("keys: {:?}", SubspaceModule::get_keys(netuid));
// 				info!("uids: {:?}", SubspaceModule::get_uids(netuid));
// 				let key: U256 = U256::from(i);
// 				info!(
// 					"Before Registered: {:?} -> {:?}",
// 					key,
// 					SubspaceModule::key_registered(netuid, &key)
// 				);
// 				register_module(netuid, key, stake_per_module);
// 				info!(
// 					"After Registered: {:?} -> {:?}",
// 					key,
// 					SubspaceModule::key_registered(netuid, &key)
// 				);
// 			}
// 			SubspaceModule::set_tempo(netuid, 1);
// 			SubspaceModule::set_max_allowed_weights(netuid, n);
// 			let keys = SubspaceModule::get_keys(netuid);
// 			let uids = SubspaceModule::get_uids(netuid);

// 			let weight_values: Vec<u16> = (0..n).collect();
// 			let weight_uids: Vec<u16> = (0..n).collect();

// 			for i in 0..n {
// 				SubspaceModule::set_weights(
// 					get_origin(keys[i as usize]),
// 					netuid,
// 					weight_values.clone(),
// 					weight_uids.clone(),
// 				)
// 				.unwrap();
// 			}
// 			step_block(1);
// 			check_network_stats(netuid);
// 		}
// 	});
// }

#[test]
fn test_blocks_until_epoch() {
    new_test_ext().execute_with(|| {
        // Check tempo = 0 block = * netuid = *
        assert_eq!(SubspaceModule::blocks_until_next_epoch(0, 0, 0), 1000);

        // Check tempo = 1 block = * netuid = *
        assert_eq!(SubspaceModule::blocks_until_next_epoch(0, 1, 0), 0);
        assert_eq!(SubspaceModule::blocks_until_next_epoch(1, 1, 0), 0);
        assert_eq!(SubspaceModule::blocks_until_next_epoch(0, 1, 1), 0);
        assert_eq!(SubspaceModule::blocks_until_next_epoch(1, 2, 1), 0);
        assert_eq!(SubspaceModule::blocks_until_next_epoch(0, 4, 3), 3);
        assert_eq!(SubspaceModule::blocks_until_next_epoch(10, 5, 2), 2);
        // Check general case.
        for netuid in 0..30_u16 {
            for block in 0..30_u64 {
                for tempo in 1..30_u16 {
                    assert_eq!(
                        SubspaceModule::blocks_until_next_epoch(netuid, tempo, block),
                        (block + netuid as u64) % (tempo as u64)
                    );
                }
            }
        }
    });
}

#[test]
fn test_incentives() {
    new_test_ext().execute_with(|| {
        // CONSSTANTS
        let netuid: u16 = 0;
        let n: u16 = 10;
        let _n_list: Vec<u16> = vec![10, 50, 100, 1000];
        let _blocks_per_epoch_list: u64 = 1;
        let stake_per_module: u64 = 10_000;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        // SETUP NETWORK
        register_n_modules(netuid, n, stake_per_module);
        let mut params = SubspaceModule::subnet_params(netuid);
        params.min_allowed_weights = 0;
        params.max_allowed_weights = n;
        params.tempo = 100;

        let keys = SubspaceModule::get_keys(netuid);
        let _uids = SubspaceModule::get_uids(netuid);

        // do a list of ones for weights
        let weight_uids: Vec<u16> = [1, 2].to_vec();
        // do a list of ones for weights
        let weight_values: Vec<u16> = [1, 1].to_vec();

        set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
        step_block(params.tempo);

        let incentives: Vec<u16> = SubspaceModule::get_incentives(netuid);
        let emissions: Vec<u64> = SubspaceModule::get_emissions(netuid);

        // evaluate votees
        assert!(incentives[1] > 0);
        assert!(incentives[1] == incentives[2]);
        assert!(emissions[1] == emissions[2]);

        // do a list of ones for weights
        let weight_values: Vec<u16> = [1, 2].to_vec();

        set_weights(netuid, keys[0], weight_uids.clone(), weight_values.clone());
        set_weights(netuid, keys[9], weight_uids.clone(), weight_values.clone());

        step_block(params.tempo);

        let incentives: Vec<u16> = SubspaceModule::get_incentives(netuid);
        let emissions: Vec<u64> = SubspaceModule::get_emissions(netuid);

        // evaluate votees
        let delta: u64 = 100 * params.tempo as u64;
        assert!(incentives[1] > 0);

        assert!(
            emissions[2] > 2 * emissions[1] - delta && emissions[2] < 2 * emissions[1] + delta,
            "emissions[1]: {} != emissions[2]: {}",
            emissions[1],
            emissions[2]
        );
    });
}

#[test]
fn test_trust() {
    new_test_ext().execute_with(|| {
        // CONSSTANTS
        let netuid: u16 = 0;
        let n: u16 = 10;
        let _n_list: Vec<u16> = vec![10, 50, 100, 1000];
        let _blocks_per_epoch_list: u64 = 1;
        let stake_per_module: u64 = 10_000;
        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        // SETUP NETWORK
        register_n_modules(netuid, n, stake_per_module);
        let mut params = SubspaceModule::subnet_params(netuid);
        params.min_allowed_weights = 1;
        params.max_allowed_weights = n;
        params.tempo = 100;
        params.trust_ratio = 100;

        update_params!(netuid => params.clone());

        let keys = SubspaceModule::get_keys(netuid);
        let _uids = SubspaceModule::get_uids(netuid);

        // do a list of ones for weights
        let weight_uids: Vec<u16> = [2].to_vec();
        let weight_values: Vec<u16> = [1].to_vec();

        set_weights(netuid, keys[8], weight_uids.clone(), weight_values.clone());
        // do a list of ones for weights
        let weight_uids: Vec<u16> = [1, 2].to_vec();
        let weight_values: Vec<u16> = [1, 1].to_vec();
        set_weights(netuid, keys[9], weight_uids.clone(), weight_values.clone());
        step_block(params.tempo);

        let trust: Vec<u16> = Trust::<Test>::get(netuid);
        let emission: Vec<u64> = SubspaceModule::get_emissions(netuid);

        // evaluate votees
        info!("trust: {:?}", trust);
        assert!(trust[1] as u32 > 0);
        assert!(trust[2] as u32 > 2 * (trust[1] as u32) - 10);
        // evaluate votees
        info!("trust: {emission:?}");
        assert!(emission[1] > 0);
        assert!(emission[2] > 2 * (emission[1]) - 1000);

        // assert!(trust[2] as u32 < 2*(trust[1] as u32)   );
    });
}

#[test]
fn test_founder_share() {
    new_test_ext().execute_with(|| {
        let netuid = 0;
        let n = 20;
        let initial_stake: u64 = 1000;
        let keys: Vec<U256> = (0..n).map(U256::from).collect();
        let stakes: Vec<u64> = (0..n).map(|_x| initial_stake * 1_000_000_000).collect();

        let founder_key = keys[0];
        SubspaceModule::set_max_registrations_per_block(1000);
        for i in 0..n {
            assert_ok!(register_module(netuid, keys[i], stakes[i]));
            let stake_from_vector = SubspaceModule::get_stake_to_vector(netuid, &keys[i]);
            info!("{:?}", stake_from_vector);
        }
        update_params!(netuid => { founder_share: 12 });
        let founder_share = SubspaceModule::get_founder_share(netuid);
        let founder_ratio: f64 = founder_share as f64 / 100.0;

        let subnet_params = SubspaceModule::subnet_params(netuid);

        let founder_stake_before = SubspaceModule::get_stake_for_key(netuid, &founder_key);
        info!("founder_stake_before: {founder_stake_before:?}");
        // vote to avoid key[0] as we want to see the key[0] burn
        step_epoch(netuid);
        let threshold = SubspaceModule::get_subnet_stake_threshold();
        let total_emission = SubspaceModule::calculate_network_emission(netuid, threshold)
            * subnet_params.tempo as u64;
        let expected_founder_share = (total_emission as f64 * founder_ratio) as u64;
        let expected_emission = total_emission - expected_founder_share;
        let emissions = SubspaceModule::get_emissions(netuid);
        let dividends = SubspaceModule::get_dividends(netuid);
        let incentives = SubspaceModule::get_incentives(netuid);
        let total_dividends: u64 = dividends.iter().sum::<u16>() as u64;
        let total_incentives: u64 = incentives.iter().sum::<u16>() as u64;

        let founder_dividend_emission = ((dividends[0] as f64 / total_dividends as f64)
            * (expected_emission / 2) as f64) as u64;
        let founder_incentive_emission = ((incentives[0] as f64 / total_incentives as f64)
            * (expected_emission / 2) as f64) as u64;
        let founder_emission = founder_incentive_emission + founder_dividend_emission;

        let calcualted_total_emission = emissions.iter().sum::<u64>();

        let key_stake = SubspaceModule::get_stake_for_key(netuid, &founder_key);
        let founder_total_stake = founder_stake_before + founder_emission;
        assert_eq!(
            key_stake - (key_stake % 1000),
            founder_total_stake - (founder_total_stake % 1000)
        );
        assert_eq!(
            GlobalDaoTreasury::<Test>::get(),
            expected_founder_share - 1 /* Account for rounding errors */
        );

        assert_eq!(
            expected_emission - (expected_emission % 100000),
            calcualted_total_emission - (calcualted_total_emission % 100000)
        );
    });
}

#[test]
fn test_dynamic_burn() {
    new_test_ext().execute_with(|| {
        let netuid = 0;
        let initial_stake: u64 = 1000;

        // make sure that the results won´t get affected by burn
        SubspaceModule::set_min_burn(0);

        // Create the subnet
        let subnet_key = U256::from(2050);
        assert_ok!(register_module(netuid, subnet_key, initial_stake));
        // Using the default GlobalParameters:
        // - registration target interval = 2 * tempo (200 blocks)
        // - registration target for interval = registration_target_interval / 2
        // - adjustment alpha = 0
        // - min_burn = 2 $COMAI
        // - max_burn = 250 $COMAI
        let mut params = SubspaceModule::global_params();
        params.min_burn = to_nano(2);
        params.max_burn = to_nano(250);
        params.adjustment_alpha = 0;
        SubspaceModule::set_global_params(params);

        // update the burn to the minimum
        step_block(200);

        assert!(
            SubspaceModule::get_burn(netuid) == SubspaceModule::get_min_burn(),
            "current burn: {:?}",
            SubspaceModule::get_burn(netuid)
        );

        // Register the first 1000 modules, this is 10x the registration target
        let registrations_per_block = 5;
        let n: usize = 1000;
        let stakes: Vec<u64> = (0..n).map(|_| initial_stake * 1_000_000_000).collect();
        for (i, stake) in stakes.iter().enumerate() {
            let key = U256::from(i);
            assert_ok!(register_module(netuid, key, *stake));
            if (i + 1) % registrations_per_block == 0 {
                step_block(1);
            }
        }

        // Burn is now at 11 instead of 2
        assert!(
            SubspaceModule::get_burn(netuid) == to_nano(11),
            "current burn {:?}",
            SubspaceModule::get_burn(netuid)
        );

        SubspaceModule::set_max_registrations_per_block(1000);
        // Register only 50 of the target
        let amount: usize = 50;
        for (i, &stake) in stakes.iter().enumerate().take(amount) {
            let key = U256::from(n + i);
            assert_ok!(register_module(netuid, key, stake));
        }

        step_block(200);

        // Make sure the burn correctly decreased based on demand
        assert!(
            SubspaceModule::get_burn(netuid) == 8250000000,
            "current burn: {:?}",
            SubspaceModule::get_burn(netuid)
        );
    });
}

#[test]
fn test_dao_treasury_distribution_for_subnet_owners() {
    new_test_ext().execute_with(|| {
        const STAKE: u64 = to_nano(1000);

        let general = (0, U256::from(0), STAKE * 10);
        let yuma_1 = (1, U256::from(1), STAKE * 4);
        let yuma_2 = (2, U256::from(2), STAKE * 6);
        let yuma_3 = (3, U256::from(3), STAKE);

        MinBurn::<Test>::set(0);

        assert_ok!(register_module(general.0, general.1, general.2));
        assert_ok!(register_module(yuma_1.0, yuma_1.1, yuma_1.2));
        assert_ok!(register_module(yuma_2.0, yuma_2.1, yuma_2.2));
        assert_ok!(register_module(yuma_3.0, yuma_3.1, yuma_3.2));

        update_params!(general.0 => { founder_share: 50, tempo: 100 });
        update_params!(yuma_1.0 => { tempo: 200 });
        update_params!(yuma_2.0 => { tempo: 200 });
        SubnetStakeThreshold::<Test>::set(Percent::from_percent(15));
        DaoTreasuryDistribution::<Test>::set(Percent::from_percent(50));
        let founder_ratio = 2;
        let treasury_distribution = 2;

        let threshold = SubspaceModule::get_subnet_stake_threshold();
        let total_emission = SubspaceModule::calculate_network_emission(general.0, threshold) * 100;

        step_epoch(general.0);

        let expected_founder_share = total_emission / founder_ratio;
        let expected_distribution @ expected_treasury =
            (expected_founder_share / treasury_distribution) as f64;

        assert_eq!(GlobalDaoTreasury::<Test>::get(), expected_treasury as u64);
        let total_yuma_stake = (yuma_1.2 + yuma_2.2) as f64;
        assert_eq!(
            SubspaceModule::get_balance_u64(&yuma_1.1) - 1,
            (expected_distribution * (yuma_1.2 as f64 / total_yuma_stake)) as u64
        );
        assert_eq!(
            SubspaceModule::get_balance_u64(&yuma_2.1) - 1,
            (expected_distribution * (yuma_2.2 as f64 / total_yuma_stake)) as u64
        );
    });
}
