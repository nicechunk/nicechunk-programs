# Treasury Swap

Treasury Swap is the fixed-rate `SOL <-> NCK` exchange inside the NiceChunk Market domain. It is not an AMM and it does not discover a market price. The NICECHUNK treasury publishes a rate on chain, supplies both reserves, and may pause the exchange.

## Custody Model

A normal wallet cannot authorize an automatic withdrawal unless it signs every user trade. Treasury Swap therefore uses four deterministic PDAs under the Market program:

| PDA | Seed | Purpose |
| --- | --- | --- |
| State | `treasury-swap-v1` | Rate, limits, fee, pause flag, revision, and cumulative totals |
| Authority | `treasury-swap-authority-v1` | Signs NCK transfers from the token vault |
| SOL vault | `treasury-swap-sol-v1` | Program-owned SOL reserve |
| NCK vault | `treasury-swap-nck-v1` | SPL Token account owned by the authority PDA |

The immutable administrator is the NICECHUNK treasury wallet `CtPV2vmqNNwUSfMu5nz58ZtMPy6ZvxL4LyNdPHVW7WvF`. Only that wallet can initialize, configure, fund, or withdraw reserves. User swaps require no treasury hot-wallet signature and settle both legs atomically. The SOL vault always retains the live Rent sysvar minimum for a zero-data account; this rent is not advertised as exchange liquidity.

## Price and Rounding

`lamports_per_nck` stores the number of lamports paid for one complete NCK (`1,000,000` NCK base units).

- `SOL -> NCK`: `floor(lamports_in * 1,000,000 / lamports_per_nck)`
- `NCK -> SOL`: `floor(nck_units_in * lamports_per_nck / 1,000,000)`
- Any configured fee is deducted from output and rounded down.

Both directions round in favor of the reserve. Repeated round trips cannot create value from integer dust.

## User Safety

Each swap payload commits to:

- exact input amount;
- minimum output amount;
- expected configuration revision;
- deadline slot.

The program rejects paused swaps, stale revisions, expired deadlines, values outside treasury limits, insufficient reserves, substituted mints/vaults/authorities, noncanonical state bytes, and every arithmetic overflow. Reserve withdrawals require the exchange to be paused. Unpausing requires both PDA reserves to cover the configured maximum single trade in each direction.

## Activation Checklist

Initialization always creates the state in paused mode. Do not unpause until all of the following have been reviewed:

1. Set an explicit Devnet rate in lamports per NCK.
2. Set minimum and maximum NCK-side trade sizes.
3. Decide the fee in basis points (`0` is allowed; maximum `1,000`).
4. Deposit initial SOL and NCK reserves from the treasury.
5. Verify every PDA, mint, live reserve, and decoded state with `sdk/nicechunk-market.ts`.
6. Run an independent security review before Mainnet use.

The rate and initial reserve amounts are economic policy, not implementation defaults. They must never be guessed by a deployment script or browser client.

## Administration

[`scripts/manage-treasury-swap.ts`](../scripts/manage-treasury-swap.ts) prepares initialization, configuration, funding, and optional activation. It accepts only integer base units, verifies the Devnet genesis hash and fixed administrator, and defaults to a read-only dry-run. It never contains or prints a treasury secret key.

Example dry-run only:

```bash
node scripts/manage-treasury-swap.ts \
  --rpc-url "$SOLANA_RPC_URL" \
  --lamports-per-nck <explicit-rate> \
  --minimum-nck-units <explicit-minimum> \
  --maximum-nck-units <explicit-maximum> \
  --fee-bps <explicit-fee> \
  --deposit-sol-lamports <explicit-sol-reserve> \
  --deposit-nck-units <explicit-nck-reserve>
```

Execution additionally requires `--execute`, a permission-restricted treasury keypair path, and the exact administrator confirmation printed by `--help`. Activation is a separate opt-in with its own confirmation phrase and is rejected if projected reserves cannot cover the configured maximum trade.
