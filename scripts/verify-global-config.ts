import {
  bigintJsonReplacer,
  decodeGlobalConfig,
  deriveGlobalConfigPda,
  NICECHUNK_CORE_PROGRAM_ID,
} from "../sdk/nicechunk-core.ts";
import { clusterUrl, connection, nckMint, programId } from "./core-script-utils.ts";

const selectedProgramId = programId() ?? NICECHUNK_CORE_PROGRAM_ID;
const expectedNckMint = nckMint();
const [globalConfig] = deriveGlobalConfigPda(selectedProgramId);
const account = await connection().getAccountInfo(globalConfig, "confirmed");

if (!account) {
  throw new Error(`GlobalConfig account not found: ${globalConfig.toBase58()}`);
}
if (!account.owner.equals(selectedProgramId)) {
  throw new Error(`Unexpected GlobalConfig owner: ${account.owner.toBase58()}`);
}

const decoded = decodeGlobalConfig(account.data);
if (!decoded.nckMint.equals(expectedNckMint)) {
  throw new Error(`Unexpected NCK mint: ${decoded.nckMint.toBase58()} expected ${expectedNckMint.toBase58()}`);
}

console.log(JSON.stringify({
  clusterUrl: clusterUrl(),
  programId: selectedProgramId.toBase58(),
  globalConfig: globalConfig.toBase58(),
  owner: account.owner.toBase58(),
  lamports: account.lamports,
  decoded: {
    ...decoded,
    nckMint: decoded.nckMint.toBase58(),
    developmentWallet: decoded.developmentWallet.toBase58(),
    worldSeed: decoded.worldSeed.toString("hex"),
    terrainConfigHash: decoded.terrainConfigHash.toString("hex"),
    resourceRuleHash: decoded.resourceRuleHash.toString("hex"),
    clientWorldConfigHash: decoded.clientWorldConfigHash.toString("hex"),
  },
}, bigintJsonReplacer, 2));
