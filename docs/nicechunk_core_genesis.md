# NiceChunk Genesis Core v0

`nicechunk_core` is the immutable genesis config layer for NiceChunk.

Future gameplay systems such as crafting, inventory, chunk state, world knowledge and reputation will be added in separate programs or future modules after PDA specs are finalized.

This program does not implement element minting, item minting, inventory, chunk state, governance books, reputation, pause, withdraw, or admin config updates.

## Program

Program name: `nicechunk_core`

Current program id in source:

```text
9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu
```

Before deployment, make sure `declare_id!` matches:

```bash
solana-keygen pubkey target/deploy/nicechunk_core-keypair.json
```

## Cluster Feature

Default feature is `devnet`.

Use explicit features when building:

```bash
cargo build-sbf --no-default-features --features devnet
cargo build-sbf --no-default-features --features testnet
cargo build-sbf --no-default-features --features mainnet
```

For current development deployment, use **devnet**.

Do not close upgrade authority during devnet/testnet work. Close upgrade authority only after audit, SDK integration, frontend integration, and PDA specs are stable.

## Devnet NCK Mint

Do not use the mainnet NCK mint on devnet/testnet.

Current devnet NCK mint:

```text
HSnWF5kjkWVrceW2SaSskScuLveUZE4gpthZ2ZXRPQPo
```

It was created to mirror mainnet NCK's core mint parameters:

- decimals = `6`
- genesis supply = `1,000,000,000 NCK`
- base units = `1_000_000_000_000_000`
- mint authority disabled
- freeze authority disabled
- program id = `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`

If you intentionally recreate the devnet mint, update `programs/nicechunk_core/src/cluster_config.rs`:

```rust
#[cfg(all(feature = "devnet", not(feature = "mainnet"), not(feature = "testnet")))]
pub const NCK_MINT: Pubkey = pubkey!("YOUR_NEW_DEVNET_NCK_MINT");
```

Also export the same mint for scripts:

```bash
export NCK_MINT=<YOUR_NEW_DEVNET_NCK_MINT>
```

## Instruction Layout

Instruction: `InitializeGlobalConfig`

```text
instruction_data: [0]
```

Accounts:

| Index | Account | Writable | Signer | Description |
|---:|---|---|---|---|
| 0 | payer | yes | yes | Pays rent and initialization transaction costs |
| 1 | global_config | yes | no | PDA derived from `["global-config"]` |
| 2 | nck_mint | no | no | SPL Token mint for current cluster |
| 3 | system_program | no | no | `11111111111111111111111111111111` |

## PDA

GlobalConfig PDA:

```text
seeds = ["global-config"]
program_id = nicechunk_core program id
```

TypeScript:

```ts
const [globalConfig, bump] = PublicKey.findProgramAddressSync(
  [Buffer.from("global-config")],
  programId,
);
```

## GlobalConfig Binary Layout

All integers are little-endian.

| Offset | Field | Type | Length |
|---:|---|---|---:|
| 0 | magic | `[u8;8]` | 8 |
| 8 | version | `u16` | 2 |
| 10 | global_config_bump | `u8` | 1 |
| 11 | sealed | `u8` | 1 |
| 12 | nck_mint | `Pubkey` | 32 |
| 44 | nck_decimals | `u8` | 1 |
| 45 | nck_genesis_supply | `u64` | 8 |
| 53 | development_wallet | `Pubkey` | 32 |
| 85 | world_id | `u16` | 2 |
| 87 | world_seed | `[u8;32]` | 32 |
| 119 | terrain_config_hash | `[u8;32]` | 32 |
| 151 | resource_rule_hash | `[u8;32]` | 32 |
| 183 | client_world_config_hash | `[u8;32]` | 32 |
| 215 | starter_pack_price_lamports | `u64` | 8 |
| 223 | genesis_pass_price_lamports | `u64` | 8 |
| 231 | starter_pack_max_per_wallet | `u8` | 1 |
| 232 | genesis_pass_max_per_wallet | `u8` | 1 |
| 233 | genesis_pass_max_supply | `u32` | 4 |
| 237 | guardian_stake_amount | `u64` | 8 |
| 245 | guardian_tax_bps | `u16` | 2 |
| 247 | protocol_fee_bps | `u16` | 2 |
| 249 | market_fee_bps | `u16` | 2 |
| 251 | slash_bps | `u16` | 2 |
| 253 | sol_to_liquidity_bps | `u16` | 2 |
| 255 | sol_to_reward_bps | `u16` | 2 |
| 257 | sol_to_development_bps | `u16` | 2 |
| 259 | chunk_size | `u16` | 2 |
| 261 | section_height | `u16` | 2 |
| 263 | min_build_y | `i16` | 2 |
| 265 | max_build_y | `i16` | 2 |
| 267 | max_terrain_height | `i16` | 2 |
| 269 | sea_level | `i16` | 2 |
| 271 | guardian_region_size_chunks | `u16` | 2 |
| 273 | guardian_realtime_radius_chunks | `u16` | 2 |
| 275 | mine_cooldown_slots | `u16` | 2 |
| 277 | genesis_slot | `u64` | 8 |
| 285 | created_at | `i64` | 8 |

Total length: `293` bytes.

## Devnet Deployment Steps

Set cluster:

```bash
solana config set --url devnet
solana balance
```

Confirm program id:

```bash
solana-keygen pubkey target/deploy/nicechunk_core-keypair.json
```

If it differs from `declare_id!`, update `programs/nicechunk_core/src/lib.rs`, then rebuild.

Build:

```bash
cargo build-sbf --no-default-features --features devnet
```

Check size and rent:

```bash
wc -c target/deploy/nicechunk_core.so
solana rent $(wc -c < target/deploy/nicechunk_core.so)
```

Deploy:

```bash
solana program deploy target/deploy/nicechunk_core.so
```

Do not close upgrade authority on devnet.

## Initialization

Environment:

```bash
export CLUSTER_URL=https://api.devnet.solana.com
export PAYER_KEYPAIR=$HOME/.config/solana/id.json
export NICECHUNK_CORE_PROGRAM_ID=<DEPLOYED_PROGRAM_ID>
export NCK_MINT=HSnWF5kjkWVrceW2SaSskScuLveUZE4gpthZ2ZXRPQPo
```

Derive PDA:

```bash
node scripts/derive-pdas.ts
```

Initialize:

```bash
node scripts/init-global-config.ts
```

The older compatibility entry also points to the devnet-safe script:

```bash
node scripts/initialize-global-config.ts
```

## Verification

```bash
node scripts/verify-global-config.ts
solana account <GLOBAL_CONFIG_PDA>
```

Expected:

- owner equals deployed `nicechunk_core` program id
- data length is `293`
- magic is `NCKCFG01`
- `nck_decimals = 6`
- `nck_genesis_supply = 1000000000000000`
- no admin, update, pause, or withdraw instruction exists

## Dust Safety

GlobalConfig initialization is safe against PDA dusting:

- if PDA owner is current program, initialization fails as already initialized
- otherwise PDA must be system-owned and have zero data length
- if PDA already has lamports, the program tops up rent if needed, then `allocate`s and `assign`s the PDA
- lamports alone no longer block initialization
