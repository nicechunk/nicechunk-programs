# NiceChunk MagicBlock ER Single Chunk Bridge

This integration is scoped to one active chunk first.

## Goal

- Delegate one `nicechunk_chunk` PDA to MagicBlock Ephemeral Rollups.
- Submit block break and block placement changes through Magic Router.
- Subscribe to the same chunk PDA over MagicBlock websocket.
- Apply received deltas immediately in the client so nearby players in the same chunk see block changes quickly.

## Devnet Only

Current NiceChunk deployment target is Solana devnet.

Do not use mainnet RPC, mainnet payer keypairs, or mainnet NCK addresses for this ER test.

## Programs

```txt
nicechunk_core   = 9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu
nicechunk_player = oeaRMVnPoV4iENnGCCtaEeRxU5be515s4YYe6aXQuKe
nicechunk_chunk  = 7JD6kASAfQeiVLUi51mrfWSbeh96ntRJnRiFQKCqUVhn
MagicBlock DLP   = DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh
Magic Router     = https://devnet-router.magicblock.app
```

## Chunk Instruction

`nicechunk_chunk` adds:

```txt
Instruction: DelegateChunkToMagicBlockER
instruction_data:
0      u8   tag = 2
1..5   i32  chunk_x
5..9   i32  chunk_z
9..13  u32  commit_frequency_ms
```

Accounts:

```txt
0. payer                 writable signer
1. chunk                 writable PDA ["chunk", global_config, chunk_x, chunk_z]
2. global_config         readonly
3. owner_program         readonly nicechunk_chunk program id
4. delegate_buffer       writable PDA ["buffer", chunk] under nicechunk_chunk
5. delegation_record     writable PDA ["delegation", chunk] under MagicBlock DLP
6. delegation_metadata   writable PDA ["delegation-metadata", chunk] under MagicBlock DLP
7. delegation_program    readonly MagicBlock DLP
8. system_program        readonly
```

The program creates the chunk PDA if needed, copies its state into the delegate buffer, assigns the chunk PDA to the MagicBlock Delegation Program, and invokes DLP delegate-with-any-validator.

## Client Runtime

The browser integration is intentionally single chunk:

- Subscribe only to the player's current chunk.
- When the player moves into another chunk, unsubscribe from the previous chunk and subscribe to the new one.
- Do not re-enable multi-chunk polling on public RPC.
- For high frequency block changes, the wallet signs once to create a short-lived `PlayerSession` PDA. A temporary session key then signs break/place transactions without prompting the wallet for every block.

Session PDA:

```txt
seeds = ["session", owner_wallet, session_authority]
owner = nicechunk_player
```

The session stores the owner wallet, session authority, player profile, global config, allowed action bitmask, expiry time and max action budget. The current chunk program validates expiry and allowed actions before writing the chunk delta.

Enable local testing manually after the devnet chunk program has been redeployed:

```js
localStorage.setItem("nicechunk.magicblockER", "1")
```

Disable:

```js
localStorage.removeItem("nicechunk.magicblockER")
```

## Commands

Build:

```bash
cargo build-sbf --no-default-features --features devnet
npm run build
```

Delegate one chunk:

```bash
PAYER_KEYPAIR=/path/to/devnet-payer.json \
CHUNK_X=0 \
CHUNK_Z=0 \
COMMIT_FREQUENCY_MS=250 \
npm run chunk:delegate-er
```

Subscribe from a terminal:

```bash
CHUNK_X=0 CHUNK_Z=0 npm run chunk:subscribe-er
```

Record a block change:

```bash
PAYER_KEYPAIR=/path/to/devnet-payer.json \
CHUNK_X=0 CHUNK_Z=0 LOCAL_X=1 BLOCK_Y=2 LOCAL_Z=1 \
PREVIOUS_BLOCK_ID=1 NEW_BLOCK_ID=0 ACTION=1 \
npm run chunk:record-block-change
```

## Current Deployment Note

The code is ready locally, but redeploying `nicechunk_chunk` requires a working devnet RPC. Public devnet RPC endpoints were rate-limited or timed out during this pass. Do not enable the browser ER switch in production until `nicechunk_chunk` is redeployed and one chunk is successfully delegated.
