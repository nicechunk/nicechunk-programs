import { PublicKey, SystemProgram } from "@solana/web3.js";
import assert from "assert";
import fs from "fs";

const PROGRAM_ID = new PublicKey("9EhMCRYMJej1F21KzaA5Zao3khGGc5aJbDGbnxaogQHu");
const NCK_MINT = new PublicKey("HSnWF5kjkWVrceW2SaSskScuLveUZE4gpthZ2ZXRPQPo");
const DEVELOPMENT_WALLET = new PublicKey("CtPV2vmqNNwUSfMu5nz58ZtMPy6ZvxL4LyNdPHVW7WvF");
const NCK = 1_000_000;
const GLOBAL_CONFIG_LEN = 293;

function pda(seed: string): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([Buffer.from(seed)], PROGRAM_ID);
}

function generatedArgs() {
  const generated = JSON.parse(fs.readFileSync("scripts/generated-global-config.json", "utf8"));
  return generated.initGlobalConfigArgs;
}

describe("nicechunk_core native", () => {
  it("derives the immutable GlobalConfig PDA", () => {
    const [globalConfig] = pda("global-config");

    assert.equal(globalConfig.toBase58(), "46bTKGThh96ChxJEmcKz6GvudGi3d7YDiAFtVhKb2Y5f");
    assert.equal(SystemProgram.programId.toBase58(), "11111111111111111111111111111111");
  });

  it("uses fixed genesis constants from the generated config", () => {
    const args = generatedArgs();

    assert.equal(args.nckMint, NCK_MINT.toBase58());
    assert.equal(args.nckDecimals, 6);
    assert.equal(args.nckGenesisSupply, 1_000_000_000 * NCK);
    assert.equal(args.developmentWallet, DEVELOPMENT_WALLET.toBase58());
    assert.equal(args.starterPackPriceLamports, 100_000_000);
    assert.equal(args.genesisPassPriceLamports, 1_000_000_000);
    assert.equal(args.guardianStakeAmount, 100_000 * NCK);
    assert.equal(args.solToLiquidityBps + args.solToRewardBps + args.solToDevelopmentBps, 10_000);
    assert.equal(args.chunkSize, 16);
    assert.equal(args.seaLevel, 2);
  });

  it("documents the native GlobalConfig binary layout", () => {
    const args = generatedArgs();
    const layout = {
      magic: [0, 8],
      version: [8, 10],
      globalConfigBump: [10, 11],
      sealed: [11, 12],
      nckMint: [12, 44],
      nckDecimals: [44, 45],
      nckGenesisSupply: [45, 53],
      developmentWallet: [53, 85],
      worldId: [85, 87],
      worldSeed: [87, 119],
      terrainConfigHash: [119, 151],
      resourceRuleHash: [151, 183],
      clientWorldConfigHash: [183, 215],
      starterPackPriceLamports: [215, 223],
      genesisPassPriceLamports: [223, 231],
      starterPackMaxPerWallet: [231, 232],
      genesisPassMaxPerWallet: [232, 233],
      genesisPassMaxSupply: [233, 237],
      guardianStakeAmount: [237, 245],
      guardianTaxBps: [245, 247],
      protocolFeeBps: [247, 249],
      marketFeeBps: [249, 251],
      slashBps: [251, 253],
      solToLiquidityBps: [253, 255],
      solToRewardBps: [255, 257],
      solToDevelopmentBps: [257, 259],
      chunkSize: [259, 261],
      sectionHeight: [261, 263],
      minBuildY: [263, 265],
      maxBuildY: [265, 267],
      maxTerrainHeight: [267, 269],
      seaLevel: [269, 271],
      guardianRegionSizeChunks: [271, 273],
      guardianRealtimeRadiusChunks: [273, 275],
      mineCooldownSlots: [275, 277],
      genesisSlot: [277, 285],
      createdAt: [285, 293],
    };

    assert.equal(layout.createdAt[1], GLOBAL_CONFIG_LEN);
    assert.equal(args.worldSeed.length, 32);
    assert.equal(args.terrainConfigHash.length, 32);
    assert.equal(args.resourceRuleHash.length, 32);
    assert.equal(args.clientWorldConfigHash.length, 32);
  });
});
