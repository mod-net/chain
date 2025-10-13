// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use crate::{AccountId, BalancesConfig, RuntimeGenesisConfig, SudoConfig};
use alloc::{vec, vec::Vec};
use frame_support::build_struct_json_patch;
use serde_json::Value;
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
use sp_consensus_grandpa::AuthorityId as GrandpaId;
use sp_genesis_builder::{self, PresetId};
use sp_keyring::Sr25519Keyring;
use sp_core::crypto::Ss58Codec;

// Returns the genesis config presets populated with given parameters.
fn testnet_genesis(
    initial_authorities: Vec<(AuraId, GrandpaId)>,
    endowed_accounts: Vec<AccountId>,
    root: AccountId,
) -> Value {
    build_struct_json_patch!(RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|k| (k, 1u128 << 60))
                .collect::<Vec<_>>(),
        },
        aura: pallet_aura::GenesisConfig {
            authorities: initial_authorities
                .iter()
                .map(|(a, _): &(AuraId, GrandpaId)| a.clone())
                .collect::<Vec<AuraId>>(),
        },
        grandpa: pallet_grandpa::GenesisConfig {
            authorities: initial_authorities
                .iter()
                .map(|(_, g): &(AuraId, GrandpaId)| (g.clone(), 1))
                .collect::<Vec<(GrandpaId, u64)>>(),
        },
        sudo: SudoConfig { key: Some(root) },
    })
}

/// Return the public testnet genesis config (Modnet Testnet).
///
/// Note: This is initially seeded with dev keyring authorities and sudo for convenience
/// during bootstrap. For a real public testnet, generate a chainspec JSON from this preset
/// and replace authorities and sudo with your production testnet keys and multisig address.
pub fn modnet_testnet_config_genesis() -> Value {
    // Helper to decode SS58 string into AccountId
    fn id(s: &str) -> AccountId { AccountId::from_ss58check(s).expect("valid ss58 address") }

    // Constants in base units (12 decimals => 10^12)
    const UNIT: u128 = 1_000_000_000_000u128;

    // Authorities: use provided keys
    let aura_id: AuraId = AuraId::from(sp_core::sr25519::Public::from_ss58check(
        "5Fga63pnkp2JDGudFzpdWNzq5CwNgbS8EUTT36DKzKJi8L7p",
    )
    .expect("valid sr25519 aura ss58"));
    let grandpa_id: GrandpaId = GrandpaId::from(sp_core::ed25519::Public::from_ss58check(
        "5HF6Koc628YWoAreCmaswgesyAdcVi1MyixPbNQEz4M3xpDm",
    )
    .expect("valid ed25519 grandpa ss58"));
    let initial_authorities = vec![(aura_id, grandpa_id)];

    // Specific allocations requested
    let signatories = vec![
        (id("5G9MCPkRmbYKvRwSog6wnfGsa474mZ7E6gyYAFjPgJMDczhq"), 100_000u128 * UNIT),
        (id("5Fqfm4drTEfBdCmjnCSTQpxWLg82UgDP3R7zKNnqFFj2GpkY"), 100_000u128 * UNIT),
        (id("5F27CcXGCpHE6ZLWV1Qy2EjNro9byxsAYzQT1kpjNwnGrguJ"), 100_000u128 * UNIT),
    ];

    let sudo_account = id("5GRgCZhCtC4dC2QsgxagTB4cZ7gJWEgVtMTbBTkFTVDsVTwC");
    let sudo_allocation = (sudo_account.clone(), 10_000_000u128 * UNIT);

    // Faucet allocation will be added when address is provided.

    // Combine all balances (signatories + sudo)
    let mut balances = signatories;
    balances.push(sudo_allocation);

    build_struct_json_patch!(RuntimeGenesisConfig {
        balances: BalancesConfig {
            balances: balances,
        },
        aura: pallet_aura::GenesisConfig {
            authorities: initial_authorities
                .iter()
                .map(|(a, _): &(AuraId, GrandpaId)| a.clone())
                .collect::<Vec<AuraId>>(),
        },
        grandpa: pallet_grandpa::GenesisConfig {
            authorities: initial_authorities
                .iter()
                .map(|(_, g): &(AuraId, GrandpaId)| (g.clone(), 1))
                .collect::<Vec<(GrandpaId, u64)>>(),
        },
        sudo: SudoConfig { key: Some(sudo_account) },
    })
}

/// Return the development genesis config.
pub fn development_config_genesis() -> Value {
    testnet_genesis(
        vec![(
            sp_keyring::Sr25519Keyring::Alice.public().into(),
            sp_keyring::Ed25519Keyring::Alice.public().into(),
        )],
        vec![
            Sr25519Keyring::Alice.to_account_id(),
            Sr25519Keyring::Bob.to_account_id(),
            Sr25519Keyring::AliceStash.to_account_id(),
            Sr25519Keyring::BobStash.to_account_id(),
        ],
        sp_keyring::Sr25519Keyring::Alice.to_account_id(),
    )
}

/// Return the local genesis config preset.
pub fn local_config_genesis() -> Value {
    testnet_genesis(
        vec![
            (
                sp_keyring::Sr25519Keyring::Alice.public().into(),
                sp_keyring::Ed25519Keyring::Alice.public().into(),
            ),
            (
                sp_keyring::Sr25519Keyring::Bob.public().into(),
                sp_keyring::Ed25519Keyring::Bob.public().into(),
            ),
        ],
        Sr25519Keyring::iter()
            .filter(|v| v != &Sr25519Keyring::One && v != &Sr25519Keyring::Two)
            .map(|v| v.to_account_id())
            .collect::<Vec<_>>(),
        Sr25519Keyring::Alice.to_account_id(),
    )
}

/// Provides the JSON representation of predefined genesis config for given `id`.
pub fn get_preset(id: &PresetId) -> Option<Vec<u8>> {
    let patch = match id.as_ref() {
        sp_genesis_builder::DEV_RUNTIME_PRESET => development_config_genesis(),
        sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET => local_config_genesis(),
        // Custom Modnet public testnet preset
        "modnet_testnet" => modnet_testnet_config_genesis(),
        _ => return None,
    };
    Some(
        serde_json::to_string(&patch)
            .expect("serialization to json is expected to work. qed.")
            .into_bytes(),
    )
}

/// List of supported presets.
pub fn preset_names() -> Vec<PresetId> {
    vec![
        PresetId::from(sp_genesis_builder::DEV_RUNTIME_PRESET),
        PresetId::from(sp_genesis_builder::LOCAL_TESTNET_RUNTIME_PRESET),
        // Expose the Modnet public testnet preset to the builder
        PresetId::from("modnet_testnet"),
    ]
}
