# nicechunk-programs Changelog

## 2026-10-19 — Starter tool gathering and crafting

- Add narrow Chunk bare-hand tags 22/23 with canonical leaf/surface validation and replay-protected pebble pickup.
- Add fixed Backpack tag 17 recipe: 1 Gravel + 2 Twigs creates one canonical axe with 99 durability.
- Bound batch inventory scratch allocation and skip redundant full compaction; preserve previous storage outcomes.
- Retain authenticated equipment wear, existing authorization and failure atomicity.


All notable public changes for this split repository should be recorded here.

This repository is generated from the main NiceChunk working tree. Make source changes in the main tree, regenerate the split repositories, then commit and push this split separately.

## Release Note Rules

- Use English for public GitHub changelog entries and release notes.
- Reference the main-tree commit and this split repository commit.
- List validation commands that were run for this split or its source surface.
- Do not include private server addresses, credentials, deployment-only scripts, raw production logs, wallet secrets, or unreleased private infrastructure details.
- Mark manual release gates as deferred unless evidence exists for the exact commit under review.

## Unreleased

- Removed the starter iron pickaxe balance and grant from PlayerProfile.
  Player and Chunk mining accept only real Equipment slots 0 through 8;
  selector 255 no longer grants or consumes a synthetic tool. Profile length
  is unchanged, reserved bytes initialize to zero, and crafted tools keep the
  fixed atomic three-point mining cost. Player and Chunk are deployed to Devnet
  with finalized executable-byte, authority and buffer-closure verification.
- Bind resource mining to a fixed three-point tool durability debit in the
  Player program, authenticated by the canonical mining progress PDA. Mining,
  rewards and wear commit atomically; paid structural continuations reuse their
  primary action. Successful mining returns the authoritative equipment balance.
  Legacy uncharged mining is retired.
  Validated with native tests, complete local SBF replays, signed authority
  cases and public read-only RPC mining simulations after the Devnet upgrade.
- Reduced repeated placement biome sampling with a bounded, fully keyed exact
  noise sample cache and initialized noise state only when generation is used.
  Fixed-width active-record slices reduce scan overhead while preserving all
  record decoding and duplicate checks. Validated with 55 Chunk unit tests,
  219,792 generation comparisons, 160 complete local SBF placement replays and
  signed receiver-authority cases. Deployed to Devnet and verified by public
  RPC simulations and finalized executable-byte checks.
- Removed repeated fixed global-config PDA derivation, placement CPI PDA
  rederivation, derived-coordinate checks and immediate header redecoding.
  Same-chunk broken-state queries now share one authenticated record scan.
  Session, canonical-address, account-owner, foundation and inventory checks
  remain enforced, including independent CPI receiver authority checks.
  Validated with 54 Chunk unit tests, generation comparisons, complete SBF
  placement replays and signed authority-boundary cases. Deployed to Devnet;
  public RPC simulations verified successful and rejected placement paths.
- Further reduced placement compute with byte-exact coordinate hash expansion,
  shared noise hash prefixes, salt-banked lattice caches, zero-weight coastal
  evaluation and conservative outer-canopy pruning. A private prepared vacancy
  proof reuses one complete placed-record scan for the target, same-chunk support
  and final append; inventory CPIs retain read-only access to that account.
  Instruction and account layouts remain compatible. Validated with
  `cargo test --offline -p nicechunk_chunk --lib`, generation comparisons and
  complete local SBF placement transactions. This follow-up was deployed to Devnet and verified with public RPC simulations.
- Reduced placement validation to canonical occupancy/support predicates and
  authenticated chunk deltas before terrain generation. Shared bounded caches
  reuse noise lattice hashes, terrain factors and tree profiles; conservative
  growth bounds reject impossible tree candidates before height generation.
  Canonical seed hash prefixes and zero-weight terrain branches avoid redundant
  work without changing world results, instruction layouts or inventory checks.
- Reused immutable terrain heights across placement target and support checks
  within one instruction. Dense tree validation could exceed the transaction
  compute limit before inventory consumption. The bounded cache preserves
  canonical generation and does not cache player modifications or change the ABI.
- Place owned equipment resource stacks even when past split/merge operations
  left rounded mass differences. Placement derives canonical mass from volume
  and the authoritative density table, updates the remainder and retains exact
  snapshot, custody, count and volume checks. Validated with player, chunk,
  backpack and game unit tests plus executable SBF regression cases.

### Split Scope

NiceChunk Solana native programs and chain operation scripts.

### Current Status

- Removed the shape-derived material-capacity gate from verified forging.
  Valid designs may exceed advisory volume, durability, and mass capacities;
  selected material slots are still validated and consumed, and their quality
  still determines the forged outcome. Instruction and account layouts remain
  compatible. Validated with backpack and unified-game unit tests and an SBF
  build; live behavior requires upgrading the gameplay program.
- Generated from the main NiceChunk working tree.
- Added treasury-issued blank land contracts, now priced at a fixed `10 NCK`, with available and reserved balances in `market-user-v1`.
- Added recoverable chunk-aligned land registration through `build-site-v3` and `foundation-chunk-v3`, capped at `4,096` contracts per parcel.
- Added versioned NCM3 building manifests and shards bound to active land while retiring Blueprint inventory issuance and v2 construction discovery.
- Apache-2.0 license metadata is included in `LICENSE`, `NOTICE`, and `docs/license-status.md`.
- CI workflow publication is documented in `docs/ci-workflow-spec.md` and remains pending credentials with `workflow` scope.

### Source Anchors

Fill these fields before publishing a release note:

- Main-tree commit: `<main NiceChunk working tree commit>`
- Split repository commit: `<split repository commit>`

### Release Evidence Checklist

Before publishing a release note for this split, cite:

~~~bash
npm run validate:repo
npm run audit:split-remotes
npm run audit:maturity
~~~

If this split contains `package.json`, also run the relevant local scripts such as `npm run audit:deps`, `npm run audit:licenses`, `npm run build`, or the split-specific `validate:release` script when present.
