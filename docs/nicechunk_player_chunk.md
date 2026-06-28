# NiceChunk Player and Chunk Programs

These native Solana programs are the first gameplay bridge after `nicechunk_core`.
They are intentionally small and only cover the login/profile and block-change flow.

## Scope

`nicechunk_player`:

- Creates one public `PlayerProfile` PDA per wallet.
- Stores basic public attributes, world position, nine visible equipment slots, backpack style, and the currently equipped backpack public key.
- Does not store private backpack contents.
- Does not mint items, resources, books, or reputation.

`nicechunk_chunk`:

- Creates one `ChunkState` PDA per `(global_config, chunk_x, chunk_z)`.
- Records player-authored block changes as fixed-size deltas.
- Does not store generated terrain.
- Does not settle mining rewards or resource discovery.

## Program IDs

Current devnet build defaults:

```text
nicechunk_core   = 9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu
nicechunk_player = oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe
nicechunk_chunk  = 7JD6kASAfQeiVLUi51mrfWSbeh96ntRJnRiFQKCqUVhn
```

Use devnet for current testing. Do not send these scripts to mainnet.

## PDA Seeds

Player profile:

```text
seeds = ["player", wallet_pubkey]
program_id = nicechunk_player
```

Chunk state:

```text
seeds = ["chunk", global_config_pubkey, i32_le(chunk_x), i32_le(chunk_z)]
program_id = nicechunk_chunk
```

## Instruction Layout

### InitializePlayer

```text
instruction_data = [0]

Accounts:
0. payer          writable signer
1. player_profile writable PDA ["player", payer]
2. global_config  readonly
3. system_program readonly
```

### UpdatePlayerPosition

```text
instruction_data = [1, i32_le(x), i32_le(y), i32_le(z)]

Accounts:
0. authority      signer
1. player_profile writable PDA ["player", authority]
2. global_config  readonly
```

### SetEquipmentSlot

```text
instruction_data = [2, slot_u8, item_pubkey_32]

Accounts:
0. authority      signer
1. player_profile writable PDA ["player", authority]
2. global_config  readonly
```

### SetBackpackStyle

```text
instruction_data = [3, backpack_style_u8]

Accounts:
0. authority      signer
1. player_profile writable PDA ["player", authority]
2. global_config  readonly
```

### SetEquippedBackpack

```text
instruction_data = [5]

Accounts:
0. authority      writable signer
1. player_profile writable PDA ["player", authority]
2. backpack       readonly Backpack PDA owned by authority
3. system_program readonly
```

This instruction writes the backpack public key into `PlayerProfile`. If the
profile already has a non-default equipped backpack key, the instruction fails.
Legacy 417-byte devnet player profiles are expanded to the current 449-byte
layout when the backpack is bound.

### InitializeChunk

```text
instruction_data = [0, i32_le(chunk_x), i32_le(chunk_z)]

Accounts:
0. payer          writable signer
1. chunk          writable PDA ["chunk", global_config, chunk_x, chunk_z]
2. global_config  readonly
3. system_program readonly
```

### RecordBlockChange

The program creates the chunk PDA automatically if it does not exist.

```text
instruction_data = [
  1,
  i32_le(chunk_x),
  i32_le(chunk_z),
  local_x_u8,
  i16_le(y),
  local_z_u8,
  u16_le(previous_block_id),
  u16_le(new_block_id),
  action_u8,
  tool_slot_u8
]

Accounts:
0. authority      writable signer
1. player_profile readonly PDA ["player", authority]
2. chunk          writable PDA ["chunk", global_config, chunk_x, chunk_z]
3. global_config  readonly
4. system_program readonly
```

## Devnet Flow

Build:

```bash
cargo build-sbf --no-default-features --features devnet
```

Deploy after confirming each `declare_id!` matches its keypair:

```bash
solana-keygen pubkey target/deploy/nicechunk_player-keypair.json
solana-keygen pubkey target/deploy/nicechunk_chunk-keypair.json
solana program deploy target/deploy/nicechunk_player.so --url devnet
solana program deploy target/deploy/nicechunk_chunk.so --url devnet
```

Initialize a player profile:

```bash
PAYER_KEYPAIR=/path/to/devnet-payer.json npm run player:init
```

Record a block break in chunk `(0, 0)`:

```bash
PAYER_KEYPAIR=/path/to/devnet-payer.json \
CHUNK_X=0 CHUNK_Z=0 LOCAL_X=1 BLOCK_Y=2 LOCAL_Z=3 \
PREVIOUS_BLOCK_ID=1 NEW_BLOCK_ID=0 ACTION=1 TOOL_SLOT=0 \
npm run chunk:record-block-change
```

Derive PDAs without sending a transaction:

```bash
PLAYER_WALLET=<wallet> CHUNK_X=0 CHUNK_Z=0 npm run core:derive-pdas
```
