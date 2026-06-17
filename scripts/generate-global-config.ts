import crypto from "crypto";
import fs from "fs";
import path from "path";
import { fileURLToPath, pathToFileURL } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const projectRoot = path.resolve(__dirname, "..");

const LAMPORTS_PER_SOL = 1_000_000_000;
const NCK = 1_000_000;
const NCK_GENESIS_SUPPLY = 1_000_000_000 * NCK;
const DEVNET_NCK_MINT = "HSnWF5kjkWVrceW2SaSskScuLveUZE4gpthZ2ZXRPQPo";

function sha256Bytes(input: Buffer | string): number[] {
  return [...crypto.createHash("sha256").update(input).digest()];
}

function stableStringify(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(",")}]`;
  if (value && typeof value === "object") {
    const record = value as Record<string, unknown>;
    return `{${Object.keys(record)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stableStringify(record[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

function readJson(relativePath: string): unknown {
  return JSON.parse(fs.readFileSync(path.join(projectRoot, relativePath), "utf8"));
}

function writeJson(relativePath: string, value: unknown): void {
  const target = path.join(projectRoot, relativePath);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, `${JSON.stringify(value, null, 2)}\n`);
}

const worldConfigModule = await import(pathToFileURL(path.join(projectRoot, "src/world/config.js")).href);
const mainnet = readJson("public/mainnet.json") as {
  world?: Record<string, unknown>;
  resources?: Record<string, unknown>;
};
const resourceRules = readJson("config/resource_rules_v1.json");

const terrainConfig = {
  generator: mainnet.world?.generator ?? "voxel-seeded-terrain-v2",
  configVersion: mainnet.world?.configVersion ?? "world-002",
  chunkSize: worldConfigModule.chunkSize,
  renderDistance: worldConfigModule.renderDistance,
  detailRenderDistance: worldConfigModule.detailRenderDistance,
  cloudSectorSize: worldConfigModule.cloudSectorSize,
  cloudRenderRadius: worldConfigModule.cloudRenderRadius,
  seaLevel: worldConfigModule.seaLevel,
  waterFlowBudget: worldConfigModule.waterFlowBudget,
  cloudMinHeight: worldConfigModule.cloudMinHeight,
};

const clientWorldConfig = {
  mainnetWorld: mainnet.world ?? {},
  runtimeWorldConfig: terrainConfig,
};

const terrainConfigHash = sha256Bytes(stableStringify(terrainConfig));
const resourceRuleHash = sha256Bytes(stableStringify(resourceRules));
const clientWorldConfigHash = sha256Bytes(stableStringify(clientWorldConfig));
const worldSeed = sha256Bytes("share-the-world");

const initGlobalConfigArgs = {
  nckDecimals: 6,
  nckGenesisSupply: NCK_GENESIS_SUPPLY,
  nckMint: process.env.NCK_MINT ?? DEVNET_NCK_MINT,

  developmentWallet:
    process.env.NICECHUNK_DEVELOPMENT_WALLET ?? "CtPV2vmqNNwUSfMu5nz58ZtMPy6ZvxL4LyNdPHVW7WvF",

  worldId: 1,
  worldSeed,
  terrainConfigHash,
  resourceRuleHash,
  clientWorldConfigHash,

  starterPackPriceLamports: Math.trunc(0.1 * LAMPORTS_PER_SOL),
  genesisPassPriceLamports: LAMPORTS_PER_SOL,
  starterPackMaxPerWallet: 1,
  genesisPassMaxPerWallet: 1,
  genesisPassMaxSupply: 10_000,

  guardianStakeAmount: 100_000 * NCK,
  guardianTaxBps: 10,
  protocolFeeBps: 50,
  marketFeeBps: 100,
  slashBps: 3000,

  solToLiquidityBps: 5000,
  solToRewardBps: 3000,
  solToDevelopmentBps: 2000,

  chunkSize: worldConfigModule.chunkSize,
  sectionHeight: 16,

  minBuildY: -32,
  maxBuildY: 256,
  maxTerrainHeight: 160,
  seaLevel: worldConfigModule.seaLevel,

  guardianRegionSizeChunks: 64,
  guardianRealtimeRadiusChunks: 16,

  mineCooldownSlots: 2,
};

const output = {
  generatedBy: "scripts/generate-global-config.ts",
  notes: {
    nckBaseUnits: "1 NCK = 1_000_000 base units",
    nckMint: "Defaults to the current devnet NCK mint. Set NCK_MINT only if intentionally replacing the devnet mint.",
    seaLevel: "Extracted from src/world/config.js for the current client world.",
    resourceRuleHash: "Based on config/resource_rules_v1.json placeholder rules.",
    developmentWallet:
      "Defaults to the Nicechunk development wallet. Override NICECHUNK_DEVELOPMENT_WALLET only for a deliberate config change.",
  },
  normalizedSources: {
    terrainConfig,
    resourceRules,
    clientWorldConfig,
  },
  hashesHex: {
    worldSeed: Buffer.from(worldSeed).toString("hex"),
    terrainConfigHash: Buffer.from(terrainConfigHash).toString("hex"),
    resourceRuleHash: Buffer.from(resourceRuleHash).toString("hex"),
    clientWorldConfigHash: Buffer.from(clientWorldConfigHash).toString("hex"),
  },
  initGlobalConfigArgs,
};

writeJson("scripts/generated-global-config.json", output);
console.log(JSON.stringify(output, null, 2));
