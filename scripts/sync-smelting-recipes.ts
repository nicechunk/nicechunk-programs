import { PublicKey, SystemProgram, Transaction } from "@solana/web3.js";
import {
  createInitializeRecipeTableInstruction,
  createUpsertSmeltingRecipeInstruction,
  deriveRecipeTablePda,
  NICECHUNK_SMELTING_PROGRAM_ID,
} from "../sdk/nicechunk-smelting.ts";
import {
  BACKPACK_ITEM_CATEGORY_MATERIAL,
  BACKPACK_SLOT_KIND_BLOCK,
  BACKPACK_SLOT_KIND_ITEM,
} from "../sdk/nicechunk-backpack.ts";
import { connection, readPayerKeypair } from "./core-script-utils.ts";
import {
  recipeRequirements,
  smeltingMaterialItemCode,
  smeltingRecipeIdForMaterialId,
  smeltingRecipeTableIdForMaterialId,
  smeltingRules,
} from "../src/data/smeltingRules.js";

const BACKPACK_PACKED_Y_BITS = 9;
const BLOCK_IDS: Record<string, number> = {
  grass: 1,
  dirt: 2,
  stone: 3,
  deepStone: 4,
  sand: 5,
  gravel: 6,
  clay: 7,
  mud: 8,
  dryDirt: 9,
  saltFlat: 10,
  snow: 11,
  ice: 12,
  frozenSoil: 13,
  basalt: 14,
  ash: 15,
  bedrock: 16,
  water: 17,
  swampWater: 18,
  toxicWater: 19,
  lava: 20,
  quicksand: 21,
  trunk: 22,
  leaves: 23,
  pineTrunk: 24,
  pineLeaves: 25,
  deadWood: 26,
  giantRoot: 27,
  grassPlant: 28,
  dryGrass: 29,
  bush: 30,
  deadBush: 31,
  cactus: 32,
  reed: 33,
  swampGrass: 34,
  snowBush: 35,
  thorn: 36,
  moss: 37,
  lichen: 38,
  vine: 39,
  glowMycelium: 40,
  mushroom: 41,
  seaweed: 42,
  aquaticPlant: 43,
  coral: 44,
  deadCoral: 45,
  shellBed: 46,
  coal: 47,
};

function inputSlot(rawKey: string) {
  const blockId = BLOCK_IDS[rawKey];
  if (!blockId) throw new Error(`No block id for smelting raw key: ${rawKey}`);
  return {
    kind: BACKPACK_SLOT_KIND_BLOCK,
    category: 0,
    flags: 0,
    quantity: 1,
    resource: { worldX: 0, worldY: blockId << BACKPACK_PACKED_Y_BITS, worldZ: 0 },
    itemCode: 0,
    itemId: 0n,
    itemPda: PublicKey.default,
  };
}

function outputSlot(material: { id: string; yieldCount?: number }, recipeTable: PublicKey) {
  return {
    kind: BACKPACK_SLOT_KIND_ITEM,
    category: BACKPACK_ITEM_CATEGORY_MATERIAL,
    flags: 0,
    quantity: Math.max(1, Math.min(99, Number(material.yieldCount) || 1)),
    resource: { worldX: 0, worldY: 0, worldZ: 0 },
    itemCode: smeltingMaterialItemCode(material.id),
    itemId: BigInt(smeltingRecipeIdForMaterialId(material.id)),
    itemPda: recipeTable,
  };
}

async function sendTx(tx: Transaction, label: string) {
  const conn = connection();
  const payer = readPayerKeypair();
  tx.feePayer = payer.publicKey;
  const { blockhash, lastValidBlockHeight } = await conn.getLatestBlockhash("confirmed");
  tx.recentBlockhash = blockhash;
  tx.lastValidBlockHeight = lastValidBlockHeight;
  tx.sign(payer);
  const signature = await conn.sendRawTransaction(tx.serialize(), { skipPreflight: false, maxRetries: 3 });
  await conn.confirmTransaction({ signature, blockhash, lastValidBlockHeight }, "confirmed");
  console.log(`${label}: ${signature}`);
  return signature;
}

async function ensureRecipeTable(tableId: bigint) {
  const conn = connection();
  const payer = readPayerKeypair();
  const [recipeTable] = deriveRecipeTablePda({ tableId });
  const account = await conn.getAccountInfo(recipeTable, "confirmed");
  console.log(JSON.stringify({
    smeltingProgramId: NICECHUNK_SMELTING_PROGRAM_ID.toBase58(),
    recipeTable: recipeTable.toBase58(),
    tableId: tableId.toString(),
    authority: payer.publicKey.toBase58(),
    exists: Boolean(account?.data?.length),
  }, null, 2));

  if (!account?.data?.length) {
    await sendTx(
      new Transaction().add(createInitializeRecipeTableInstruction({ payer: payer.publicKey, tableId })),
      `initialize recipe table ${tableId}`,
    );
  }
  return recipeTable;
}

async function main() {
  const payer = readPayerKeypair();
  const tableCache = new Map<string, PublicKey>();
  for (const material of smeltingRules.materials) {
    const tableId = BigInt(smeltingRecipeTableIdForMaterialId(material.id));
    if (!tableCache.has(tableId.toString())) {
      tableCache.set(tableId.toString(), await ensureRecipeTable(tableId));
    }
    const recipeTable = tableCache.get(tableId.toString())!;
    const inputs = [];
    for (const requirement of recipeRequirements(material)) {
      for (let count = 0; count < requirement.amount; count += 1) {
        inputs.push(inputSlot(requirement.key));
      }
    }
    if (!inputs.length || inputs.length > 8) {
      throw new Error(`Recipe ${material.id} has unsupported input count ${inputs.length}`);
    }
    const recipeId = BigInt(smeltingRecipeIdForMaterialId(material.id));
    const tx = new Transaction().add(createUpsertSmeltingRecipeInstruction({
      authority: payer.publicKey,
      recipeTable,
      recipe: {
        recipeId,
        enabled: true,
        minHeatTier: material.requiredHeatTier,
        inputs,
        outputs: [outputSlot(material, recipeTable)],
      },
    }));
    await sendTx(tx, `upsert ${material.id} table ${tableId}`);
  }
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
