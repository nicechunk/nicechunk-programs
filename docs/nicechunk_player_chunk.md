# NiceChunk Player, Mining, and Skill Protocol

This document describes the current devnet protocol. NiceChunk is still on
testnet, so retired account layouts and instruction payloads are deliberately
unsupported. Clients must use the constants and builders in `sdk/` rather than
attempting to decode or migrate older accounts.

## Program IDs

```text
nicechunk_core     = 9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu
nicechunk_player   = CHZHsBCGn58ih2WrPfKSYhvCEjMPGhArTiYCH7AWWBkB
nicechunk_chunk    = GnVKn442KDTDgCyjVG7SEtCQQLjaCiLvrEZDWSU13wbj
nicechunk_game     = 6CurnvneezBuHwPUnrCiFg1QMWeUF67ufQxYebyr2UP7
nicechunk_skills   = 5gkdfmRJogdSdPrT8rvnEkPdn2N2fLBnQ6YDdegUcu3P
```

Use devnet for current testing. Do not send these scripts or transactions to
mainnet.

## Final PDA Seeds

```text
PlayerProfile     ["player-v7", owner]
PlayerSession     ["session", owner, session_authority]
ChunkBroken       ["chunk-broken", global_config, i32_le(chunk_x), i32_le(chunk_z)]
PlayerProgress    ["player-progress", global_config, owner]
FoundationChunk   ["foundation-chunk-v2", global_config, i32_le(chunk_x), i32_le(chunk_z)]
Backpack          ["backpack", owner, u64_le(backpack_id)]
PlayerSkills      ["player-skills-v2", global_config, owner]
SkillRuleTable    ["skill-rules-v2", global_config]
```

`FoundationChunk` accepts only `NCKFCI02` version 2. `PlayerSkills` accepts only
`NCKSKL02` version 2, and `SkillRuleTable` accepts only `NCKXPR02` version 2.

## Mining Action Identity

Every reward-bearing mining instruction requires a nonzero unsigned 64-bit
`action_id`. One logical player action uses one ID across all of its transactions
and retries. A normal mine, one whole-tree fell, one blast, or one multi-range
batch is one logical action.

The backpack stores the latest mining action ID, the mass before that action,
and a monotonically increasing mining sequence. Reusing the same action ID lets
split transactions store every valid reward while preventing duplicate
Precision Gathering, Burden, and Swiftness XP.

### MineBlockWithRewards

```text
instruction_data = [
  8,
  u64_le(action_id),
  i32_le(world_x),
  i16_le(world_y),
  i32_le(world_z),
  u16_le(expected_block_id)
]
```

### FellTreeWithRewards

The payload uses the same 21-byte shape as `MineBlockWithRewards`, with tag 9.
The action ID covers the complete generated tree.

### BatchMineWithRewards

```text
instruction_data = [
  20,
  u64_le(action_id),
  mode_u8,
  count_u8,
  count * [i32_le(world_x), i16_le(world_y), i32_le(world_z), u16_le(block_id)]
]
```

### RangeMineWithRewards

```text
instruction_data = [
  21,
  u64_le(action_id),
  mode_u8,
  i32_le(min_x),
  i16_le(min_y),
  i32_le(min_z),
  width_u8,
  u16_le(height),
  depth_u8,
  occupancy_bitset,
  palette_count_u8,
  sorted_u8_palette,
  packed_palette_indexes
]
```

The current limit is 640 selected blocks and eight block types. The palette is
strictly sorted, every palette entry must be used, and the compressed payload
must be canonical. This keeps a maximum range instruction plus skill sync below
Solana's 1,232-byte packet limit.

## Skill Rules

The rule table stores ten cumulative level thresholds, six generic source rules,
the Burden mining rule, and the Swiftness travel rule. Skill effects are read
from `PlayerSkills.levels`; raw counters remain in their authoritative source
accounts and are synchronized through verified PDA rules.

| Skill | Current effect | XP source |
| --- | --- | --- |
| Precision Gathering | 50% resource volume at level 0, plus 5% per level, capped at 100% | 1 XP per complete mining action |
| Burden | 50 kg at level 0, plus 10 kg per level, capped at 150 kg | `floor(pre-mine mass kg / 20) x Chebyshev Chunk distance`, distance capped at 5 and same-Chunk distance equal to 0 |
| Smelting | 0% extra output at level 0, plus 5% per level, capped at 50% | 1 XP per completed normal recipe; material-stack merge recipes grant 0 XP |
| Forging | 0% durability bonus at level 0, plus 5% per level, capped at 50% | 1 XP per successfully forged item |
| Swiftness | 100% movement speed at level 0, plus 3% per level, capped at 130% | 1 XP when consecutive verified mining coordinates are at least 160 blocks apart |
| Exploration | 0% rare extra-drop weight at level 0, plus 10% per level, capped at 100% | 1 XP for each rare extra drop that actually triggers |

Burden uses the backpack mass captured before rewards from the current action are
added. Its level thresholds are fixed at 10,000 XP per level. Precision Gathering
calculates mass only after the verified volume is known:

```text
volume_mm3 = 1,000,000 x (50 + 5 x level) / 100
mass_grams = round(volume_mm3 x density_kg_m3 / 1,000,000)
```

Batch mining, blasting, support collapse, and whole-tree felling grant Precision
Gathering XP once for the complete logical action, not once per block or retry.

## Client Transaction Order

When a source cursor has not been initialized, the client adds a baseline skill
sync before the gameplay instruction. The transaction then executes the gameplay
instruction and a final skill sync:

```text
compute budget
optional baseline SyncPlayerSkills
mining, smelting, or forging instruction
final SyncPlayerSkills
```

The baseline records existing source counters without backfilling historical XP.
The final sync observes only the counter delta created by the current action. A
mining sync also includes the verified mining coordinate and the instructions
sysvar so the Skills program can prove that a matching Chunk instruction already
ran in the same transaction.

## Validation

```bash
cargo fmt --check
cargo test --workspace
```

Build each program for devnet only after its `declare_id!` and upgrade keypair are
confirmed:

```bash
cargo build-sbf --no-default-features --features devnet
```

From the complete NiceChunk development checkout, with the root JavaScript
dependencies installed, initialize the final skill rule table with:

```bash
SOLANA_RPC_URL=https://api.devnet.solana.com \
SOLANA_KEYPAIR=/path/to/devnet-authority.json \
node scripts/initialize-skill-rules.ts
```
