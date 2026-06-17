# NiceChunk

NiceChunk contains the browser client and Solana programs for the NiceChunk voxel world.

## NiceChunk Genesis Core v0

`programs/nicechunk_core` is a native Solana program, not Anchor.

This program is the immutable genesis config layer. Future gameplay systems such as crafting, inventory, chunk state, world knowledge and reputation will be added in separate programs or future modules after PDA specs are finalized.

Current scope:

- initialize the `GlobalConfig` PDA
- validate the current-cluster NCK mint
- write fixed genesis world parameters
- expose a fixed binary layout for frontend and scripts

Out of scope for this version:

- element minting
- item minting
- inventory
- chunk state
- book voting
- reputation
- admin config updates
- pause or withdraw instructions

For devnet deployment and SDK usage, see [docs/nicechunk_core_genesis.md](docs/nicechunk_core_genesis.md).
For the player login and chunk block-change bridge, see [docs/nicechunk_player_chunk.md](docs/nicechunk_player_chunk.md).

## Devnet Core Commands

Use devnet for current deployment testing:

```bash
solana config set --url devnet
cargo build-sbf --no-default-features --features devnet
```

Current devnet NCK mint:

```text
HSnWF5kjkWVrceW2SaSskScuLveUZE4gpthZ2ZXRPQPo
```

It mirrors mainnet NCK core mint parameters: Tokenkeg SPL Token, 6 decimals, 1,000,000,000 NCK genesis supply, no mint authority, and no freeze authority.

Do not use the mainnet NCK mint on devnet/testnet.

Do not close upgrade authority during devnet/testnet work.
