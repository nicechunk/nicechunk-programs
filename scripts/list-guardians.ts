import {
  decodeGuardianRegion,
  GUARDIAN_REGION_LEN,
  GUARDIAN_STATUS_ACTIVE,
} from "../sdk/nicechunk-guardian.ts";
import {
  connection,
  guardianProgramId,
} from "./core-script-utils.ts";

const conn = connection();
const selectedGuardianProgramId = guardianProgramId();
const accounts = await conn.getProgramAccounts(selectedGuardianProgramId, {
  commitment: "confirmed",
  filters: [{ dataSize: GUARDIAN_REGION_LEN }],
});
const guardians = accounts
  .map(({ pubkey, account }) => {
    try {
      return decodeGuardianRegion(account.data, pubkey);
    } catch (_error) {
      return null;
    }
  })
  .filter((guardian) => guardian && guardian.status === GUARDIAN_STATUS_ACTIVE);

console.log(JSON.stringify({
  guardianProgramId: selectedGuardianProgramId.toBase58(),
  count: guardians.length,
  guardians,
}, (_key, value) => typeof value === "bigint" ? value.toString() : value, 2));
