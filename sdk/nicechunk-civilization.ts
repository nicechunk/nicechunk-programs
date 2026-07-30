import {
  PublicKey,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { Buffer } from "buffer";

const env = typeof process !== "undefined" ? process.env : {};

export const NICECHUNK_CIVILIZATION_PROGRAM_ID = new PublicKey(
  env.NICECHUNK_CIVILIZATION_PROGRAM_ID ?? "3MRG4UjxTK1rMq7TGM4bX1GrD8C36bQtt1RdTmJD7Jah",
);
export const NICECHUNK_CIVILIZATION_POWER_AUTHORITY = new PublicKey(
  env.NICECHUNK_CIVILIZATION_POWER_AUTHORITY ?? "9XuoVVwqP2jipt3jpJVXCSS2N2jr9vDuV3d6K73FKVud",
);

export const RULE_BOOK_SEED = "rule-book";
export const RULE_SIGNATURE_SEED = "rule-signature";
export const RULE_TALLY_SEED = "rule-tally";
export const EXECUTION_RECEIPT_SEED = "rule-execution";
export const POWER_SNAPSHOT_SEED = "power-snapshot";
export const CITIZEN_POWER_SEED = "citizen-power";
export const CIVILIZATION_ADAPTER_AUTHORITY_SEED = "civilization-adapter";

export const RULE_BOOK_MAGIC = "NCKCVR01";
export const RULE_SIGNATURE_MAGIC = "NCKCVS01";
export const RULE_TALLY_MAGIC = "NCKCVT01";
export const EXECUTION_RECEIPT_MAGIC = "NCKCVE01";
export const POWER_SNAPSHOT_MAGIC = "NCKCVP01";
export const CITIZEN_POWER_MAGIC = "NCKCVC01";

export const RULE_BOOK_LEN = 320;
export const RULE_SIGNATURE_LEN = 136;
export const RULE_TALLY_LEN = 128;
export const EXECUTION_RECEIPT_LEN = 128;
export const POWER_SNAPSHOT_LEN = 128;
export const CITIZEN_POWER_LEN = 144;

export const RULE_STATUS_PUBLISHED = 1;
export const RULE_STATUS_FINALIZED = 2;
export const RULE_STATUS_EXECUTED = 3;

export const CIVILIZATION_SIDE_AGREE = 1;
export const CIVILIZATION_SIDE_REJECT = 2;
export const CIVILIZATION_SIDE_ABSTAIN = 3;
export const CIVILIZATION_SIDE_CHALLENGE = 4;

export interface PublishCivilizationRuleInput {
  payer: PublicKey;
  ruleIdHash: Buffer | Uint8Array | string;
  textHash: Buffer | Uint8Array | string;
  patchHash: Buffer | Uint8Array | string;
  snapshotHash: Buffer | Uint8Array | string;
  targetProgram: PublicKey;
  targetPda: PublicKey;
  totalActivePower: bigint | number;
  thresholdBps: number;
  targetStatus: number;
  riskLevel: number;
  powerSnapshot?: PublicKey;
  civilizationProgramId?: PublicKey;
}

export interface SignCivilizationRuleInput {
  signer: PublicKey;
  ruleBook: PublicKey;
  snapshotHash: Buffer | Uint8Array | string;
  side: number;
  snapshotPower: bigint | number;
  citizenPower?: PublicKey;
  civilizationProgramId?: PublicKey;
}

export interface PublishPowerSnapshotInput {
  authority: PublicKey;
  snapshotHash: Buffer | Uint8Array | string;
  totalActivePower: bigint | number;
  windowStartEpoch: bigint | number;
  windowEndEpoch: bigint | number;
  expiresAt: bigint | number;
  civilizationProgramId?: PublicKey;
}

export interface SettleCitizenPowerInput {
  authority: PublicKey;
  powerSnapshot: PublicKey;
  citizen: PublicKey;
  power: bigint | number;
  civilizationProgramId?: PublicKey;
}

export interface DecodedRuleBook {
  magic: string;
  version: number;
  bump: number;
  status: number;
  author: PublicKey;
  ruleIdHash: Buffer;
  textHash: Buffer;
  patchHash: Buffer;
  snapshotHash: Buffer;
  targetProgram: PublicKey;
  targetPda: PublicKey;
  totalActivePower: bigint;
  requiredPower: bigint;
  thresholdBps: number;
  targetStatus: number;
  riskLevel: number;
  yesPower: bigint;
  noPower: bigint;
  abstainPower: bigint;
  challengePower: bigint;
  createdSlot: bigint;
  finalizedSlot: bigint;
  executedSlot: bigint;
  createdAt: bigint;
}

export interface DecodedRuleSignature {
  magic: string;
  version: number;
  bump: number;
  side: number;
  ruleBook: PublicKey;
  signer: PublicKey;
  snapshotHash: Buffer;
  snapshotPower: bigint;
  signedSlot: bigint;
  signedAt: bigint;
}

export interface DecodedRuleTally {
  magic: string;
  version: number;
  bump: number;
  thresholdMet: boolean;
  ruleBook: PublicKey;
  yesPower: bigint;
  noPower: bigint;
  abstainPower: bigint;
  challengePower: bigint;
  requiredPower: bigint;
  finalizedSlot: bigint;
  finalizedAt: bigint;
}

export interface DecodedExecutionReceipt {
  magic: string;
  version: number;
  bump: number;
  executed: boolean;
  ruleBook: PublicKey;
  executor: PublicKey;
  executedSlot: bigint;
  executedAt: bigint;
}

export interface DecodedPowerSnapshot {
  magic: string;
  version: number;
  bump: number;
  authority: PublicKey;
  snapshotHash: Buffer;
  totalActivePower: bigint;
  windowStartEpoch: bigint;
  windowEndEpoch: bigint;
  createdSlot: bigint;
  expiresAt: bigint;
  createdAt: bigint;
}

export interface DecodedCitizenPower {
  magic: string;
  version: number;
  bump: number;
  powerSnapshot: PublicKey;
  citizen: PublicKey;
  snapshotHash: Buffer;
  power: bigint;
  settledSlot: bigint;
  settledAt: bigint;
  expiresAt: bigint;
}

export function deriveRuleBookPda({
  ruleIdHash,
  programId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: {
  ruleIdHash: Buffer | Uint8Array | string;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(RULE_BOOK_SEED), hash32(ruleIdHash)],
    programId,
  );
}

export function deriveRuleSignaturePda({
  ruleBook,
  signer,
  programId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: {
  ruleBook: PublicKey;
  signer: PublicKey;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(RULE_SIGNATURE_SEED), ruleBook.toBuffer(), signer.toBuffer()],
    programId,
  );
}

export function deriveRuleTallyPda({
  ruleBook,
  programId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: {
  ruleBook: PublicKey;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(RULE_TALLY_SEED), ruleBook.toBuffer()],
    programId,
  );
}

export function deriveExecutionReceiptPda({
  ruleBook,
  programId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: {
  ruleBook: PublicKey;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(EXECUTION_RECEIPT_SEED), ruleBook.toBuffer()],
    programId,
  );
}

export function derivePowerSnapshotPda({
  snapshotHash,
  programId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: {
  snapshotHash: Buffer | Uint8Array | string;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(POWER_SNAPSHOT_SEED), hash32(snapshotHash)],
    programId,
  );
}

export function deriveCitizenPowerPda({
  powerSnapshot,
  citizen,
  programId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: {
  powerSnapshot: PublicKey;
  citizen: PublicKey;
  programId?: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(CITIZEN_POWER_SEED), powerSnapshot.toBuffer(), citizen.toBuffer()],
    programId,
  );
}

export function deriveCivilizationAdapterAuthorityPda({
  ruleBook,
  targetProgram,
}: {
  ruleBook: PublicKey;
  targetProgram: PublicKey;
}): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [Buffer.from(CIVILIZATION_ADAPTER_AUTHORITY_SEED), ruleBook.toBuffer()],
    targetProgram,
  );
}

export function createPublishCivilizationRuleInstruction(input: PublishCivilizationRuleInput): TransactionInstruction {
  const civilizationProgramId = input.civilizationProgramId ?? NICECHUNK_CIVILIZATION_PROGRAM_ID;
  const [ruleBook] = deriveRuleBookPda({ ruleIdHash: input.ruleIdHash, programId: civilizationProgramId });
  const powerSnapshot = input.powerSnapshot
    ?? derivePowerSnapshotPda({ snapshotHash: input.snapshotHash, programId: civilizationProgramId })[0];
  return new TransactionInstruction({
    programId: civilizationProgramId,
    keys: [
      { pubkey: input.payer, isSigner: true, isWritable: true },
      { pubkey: ruleBook, isSigner: false, isWritable: true },
      { pubkey: powerSnapshot, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data: Buffer.concat([Buffer.from([0]), encodePublishRuleArgs(input)]),
  });
}

export function createSignCivilizationRuleInstruction(input: SignCivilizationRuleInput): TransactionInstruction {
  const civilizationProgramId = input.civilizationProgramId ?? NICECHUNK_CIVILIZATION_PROGRAM_ID;
  const [signature] = deriveRuleSignaturePda({
    ruleBook: input.ruleBook,
    signer: input.signer,
    programId: civilizationProgramId,
  });
  const powerSnapshot = derivePowerSnapshotPda({
    snapshotHash: input.snapshotHash,
    programId: civilizationProgramId,
  })[0];
  const citizenPower = input.citizenPower
    ?? deriveCitizenPowerPda({ powerSnapshot, citizen: input.signer, programId: civilizationProgramId })[0];
  const data = Buffer.alloc(42);
  data.writeUInt8(1, 0);
  data.writeUInt8(input.side, 1);
  data.writeBigUInt64LE(BigInt(input.snapshotPower), 2);
  hash32(input.snapshotHash).copy(data, 10);
  return new TransactionInstruction({
    programId: civilizationProgramId,
    keys: [
      { pubkey: input.signer, isSigner: true, isWritable: true },
      { pubkey: input.ruleBook, isSigner: false, isWritable: false },
      { pubkey: signature, isSigner: false, isWritable: true },
      { pubkey: citizenPower, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createPublishPowerSnapshotInstruction(input: PublishPowerSnapshotInput): TransactionInstruction {
  const civilizationProgramId = input.civilizationProgramId ?? NICECHUNK_CIVILIZATION_PROGRAM_ID;
  const [powerSnapshot] = derivePowerSnapshotPda({
    snapshotHash: input.snapshotHash,
    programId: civilizationProgramId,
  });
  const data = Buffer.alloc(65);
  data.writeUInt8(4, 0);
  hash32(input.snapshotHash).copy(data, 1);
  data.writeBigUInt64LE(BigInt(input.totalActivePower), 33);
  data.writeBigUInt64LE(BigInt(input.windowStartEpoch), 41);
  data.writeBigUInt64LE(BigInt(input.windowEndEpoch), 49);
  data.writeBigInt64LE(BigInt(input.expiresAt), 57);
  return new TransactionInstruction({
    programId: civilizationProgramId,
    keys: [
      { pubkey: input.authority, isSigner: true, isWritable: true },
      { pubkey: powerSnapshot, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createSettleCitizenPowerInstruction(input: SettleCitizenPowerInput): TransactionInstruction {
  const civilizationProgramId = input.civilizationProgramId ?? NICECHUNK_CIVILIZATION_PROGRAM_ID;
  const [citizenPower] = deriveCitizenPowerPda({
    powerSnapshot: input.powerSnapshot,
    citizen: input.citizen,
    programId: civilizationProgramId,
  });
  const data = Buffer.alloc(9);
  data.writeUInt8(5, 0);
  data.writeBigUInt64LE(BigInt(input.power), 1);
  return new TransactionInstruction({
    programId: civilizationProgramId,
    keys: [
      { pubkey: input.authority, isSigner: true, isWritable: true },
      { pubkey: input.powerSnapshot, isSigner: false, isWritable: false },
      { pubkey: citizenPower, isSigner: false, isWritable: true },
      { pubkey: input.citizen, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ],
    data,
  });
}

export function createFinalizeCivilizationRuleInstruction({
  payer,
  ruleBook,
  signatures,
  civilizationProgramId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: {
  payer: PublicKey;
  ruleBook: PublicKey;
  signatures: PublicKey[];
  civilizationProgramId?: PublicKey;
}): TransactionInstruction {
  const [tally] = deriveRuleTallyPda({ ruleBook, programId: civilizationProgramId });
  return new TransactionInstruction({
    programId: civilizationProgramId,
    keys: [
      { pubkey: payer, isSigner: true, isWritable: true },
      { pubkey: ruleBook, isSigner: false, isWritable: true },
      { pubkey: tally, isSigner: false, isWritable: true },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
      ...signatures.map((pubkey) => ({ pubkey, isSigner: false, isWritable: false })),
    ],
    data: Buffer.from([2]),
  });
}

export function createExecuteCivilizationRuleReceiptInstruction({
  executor,
  ruleBook,
  adapterAuthority,
  civilizationProgramId = NICECHUNK_CIVILIZATION_PROGRAM_ID,
}: {
  executor: PublicKey;
  ruleBook: PublicKey;
  adapterAuthority?: PublicKey;
  civilizationProgramId?: PublicKey;
}): TransactionInstruction {
  const [tally] = deriveRuleTallyPda({ ruleBook, programId: civilizationProgramId });
  const [receipt] = deriveExecutionReceiptPda({ ruleBook, programId: civilizationProgramId });
  const keys = [
    { pubkey: executor, isSigner: true, isWritable: true },
    { pubkey: ruleBook, isSigner: false, isWritable: true },
    { pubkey: tally, isSigner: false, isWritable: false },
    { pubkey: receipt, isSigner: false, isWritable: true },
    { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
  ];
  if (adapterAuthority) {
    keys.push({ pubkey: adapterAuthority, isSigner: false, isWritable: false });
  }
  return new TransactionInstruction({
    programId: civilizationProgramId,
    keys,
    data: Buffer.from([3]),
  });
}

export function encodePublishRuleArgs(input: Omit<PublishCivilizationRuleInput, "payer" | "civilizationProgramId">): Buffer {
  const data = Buffer.alloc(204);
  let offset = 0;
  const bytes = (value: Buffer | Uint8Array | string): void => {
    hash32(value).copy(data, offset);
    offset += 32;
  };
  bytes(input.ruleIdHash);
  bytes(input.textHash);
  bytes(input.patchHash);
  bytes(input.snapshotHash);
  input.targetProgram.toBuffer().copy(data, offset); offset += 32;
  input.targetPda.toBuffer().copy(data, offset); offset += 32;
  data.writeBigUInt64LE(BigInt(input.totalActivePower), offset); offset += 8;
  data.writeUInt16LE(input.thresholdBps, offset); offset += 2;
  data.writeUInt8(input.targetStatus, offset++);
  data.writeUInt8(input.riskLevel, offset++);
  return data;
}

export function decodeRuleBook(data: Buffer): DecodedRuleBook {
  assertLen(data, RULE_BOOK_LEN, "RuleBook");
  let offset = 0;
  const decoded: DecodedRuleBook = {
    magic: bytes(data, offset, 8).toString("utf8"), version: 0, bump: 0, status: 0,
    author: PublicKey.default, ruleIdHash: Buffer.alloc(0), textHash: Buffer.alloc(0),
    patchHash: Buffer.alloc(0), snapshotHash: Buffer.alloc(0), targetProgram: PublicKey.default,
    targetPda: PublicKey.default, totalActivePower: 0n, requiredPower: 0n, thresholdBps: 0,
    targetStatus: 0, riskLevel: 0, yesPower: 0n, noPower: 0n, abstainPower: 0n,
    challengePower: 0n, createdSlot: 0n, finalizedSlot: 0n, executedSlot: 0n, createdAt: 0n,
  };
  offset += 8;
  decoded.version = data.readUInt16LE(offset); offset += 2;
  decoded.bump = data.readUInt8(offset++);
  decoded.status = data.readUInt8(offset++);
  decoded.author = new PublicKey(bytes(data, offset, 32)); offset += 32;
  decoded.ruleIdHash = bytes(data, offset, 32); offset += 32;
  decoded.textHash = bytes(data, offset, 32); offset += 32;
  decoded.patchHash = bytes(data, offset, 32); offset += 32;
  decoded.snapshotHash = bytes(data, offset, 32); offset += 32;
  decoded.targetProgram = new PublicKey(bytes(data, offset, 32)); offset += 32;
  decoded.targetPda = new PublicKey(bytes(data, offset, 32)); offset += 32;
  decoded.totalActivePower = data.readBigUInt64LE(offset); offset += 8;
  decoded.requiredPower = data.readBigUInt64LE(offset); offset += 8;
  decoded.thresholdBps = data.readUInt16LE(offset); offset += 2;
  decoded.targetStatus = data.readUInt8(offset++);
  decoded.riskLevel = data.readUInt8(offset++);
  decoded.yesPower = data.readBigUInt64LE(offset); offset += 8;
  decoded.noPower = data.readBigUInt64LE(offset); offset += 8;
  decoded.abstainPower = data.readBigUInt64LE(offset); offset += 8;
  decoded.challengePower = data.readBigUInt64LE(offset); offset += 8;
  decoded.createdSlot = data.readBigUInt64LE(offset); offset += 8;
  decoded.finalizedSlot = data.readBigUInt64LE(offset); offset += 8;
  decoded.executedSlot = data.readBigUInt64LE(offset); offset += 8;
  decoded.createdAt = data.readBigInt64LE(offset);
  if (decoded.magic !== RULE_BOOK_MAGIC) throw new Error(`Invalid RuleBook magic: ${decoded.magic}`);
  return decoded;
}

export function decodeRuleSignature(data: Buffer): DecodedRuleSignature {
  assertLen(data, RULE_SIGNATURE_LEN, "RuleSignature");
  let offset = 0;
  const magic = bytes(data, offset, 8).toString("utf8"); offset += 8;
  if (magic !== RULE_SIGNATURE_MAGIC) throw new Error(`Invalid RuleSignature magic: ${magic}`);
  const version = data.readUInt16LE(offset); offset += 2;
  const bump = data.readUInt8(offset++);
  const side = data.readUInt8(offset++);
  const ruleBook = new PublicKey(bytes(data, offset, 32)); offset += 32;
  const signer = new PublicKey(bytes(data, offset, 32)); offset += 32;
  const snapshotHash = bytes(data, offset, 32); offset += 32;
  const snapshotPower = data.readBigUInt64LE(offset); offset += 8;
  const signedSlot = data.readBigUInt64LE(offset); offset += 8;
  const signedAt = data.readBigInt64LE(offset);
  return { magic, version, bump, side, ruleBook, signer, snapshotHash, snapshotPower, signedSlot, signedAt };
}

export function decodeRuleTally(data: Buffer): DecodedRuleTally {
  assertLen(data, RULE_TALLY_LEN, "RuleTally");
  let offset = 0;
  const magic = bytes(data, offset, 8).toString("utf8"); offset += 8;
  if (magic !== RULE_TALLY_MAGIC) throw new Error(`Invalid RuleTally magic: ${magic}`);
  const version = data.readUInt16LE(offset); offset += 2;
  const bump = data.readUInt8(offset++);
  const thresholdMet = data.readUInt8(offset++) === 1;
  const ruleBook = new PublicKey(bytes(data, offset, 32)); offset += 32;
  const yesPower = data.readBigUInt64LE(offset); offset += 8;
  const noPower = data.readBigUInt64LE(offset); offset += 8;
  const abstainPower = data.readBigUInt64LE(offset); offset += 8;
  const challengePower = data.readBigUInt64LE(offset); offset += 8;
  const requiredPower = data.readBigUInt64LE(offset); offset += 8;
  const finalizedSlot = data.readBigUInt64LE(offset); offset += 8;
  const finalizedAt = data.readBigInt64LE(offset);
  return { magic, version, bump, thresholdMet, ruleBook, yesPower, noPower, abstainPower, challengePower, requiredPower, finalizedSlot, finalizedAt };
}

export function decodeExecutionReceipt(data: Buffer): DecodedExecutionReceipt {
  assertLen(data, EXECUTION_RECEIPT_LEN, "ExecutionReceipt");
  let offset = 0;
  const magic = bytes(data, offset, 8).toString("utf8"); offset += 8;
  if (magic !== EXECUTION_RECEIPT_MAGIC) throw new Error(`Invalid ExecutionReceipt magic: ${magic}`);
  const version = data.readUInt16LE(offset); offset += 2;
  const bump = data.readUInt8(offset++);
  const executed = data.readUInt8(offset++) === 1;
  const ruleBook = new PublicKey(bytes(data, offset, 32)); offset += 32;
  const executor = new PublicKey(bytes(data, offset, 32)); offset += 32;
  const executedSlot = data.readBigUInt64LE(offset); offset += 8;
  const executedAt = data.readBigInt64LE(offset);
  return { magic, version, bump, executed, ruleBook, executor, executedSlot, executedAt };
}

export function decodePowerSnapshot(data: Buffer): DecodedPowerSnapshot {
  assertLen(data, POWER_SNAPSHOT_LEN, "PowerSnapshot");
  let offset = 0;
  const magic = bytes(data, offset, 8).toString("utf8"); offset += 8;
  if (magic !== POWER_SNAPSHOT_MAGIC) throw new Error(`Invalid PowerSnapshot magic: ${magic}`);
  const version = data.readUInt16LE(offset); offset += 2;
  const bump = data.readUInt8(offset++);
  const authority = new PublicKey(bytes(data, offset, 32)); offset += 32;
  const snapshotHash = bytes(data, offset, 32); offset += 32;
  const totalActivePower = data.readBigUInt64LE(offset); offset += 8;
  const windowStartEpoch = data.readBigUInt64LE(offset); offset += 8;
  const windowEndEpoch = data.readBigUInt64LE(offset); offset += 8;
  const createdSlot = data.readBigUInt64LE(offset); offset += 8;
  const expiresAt = data.readBigInt64LE(offset); offset += 8;
  const createdAt = data.readBigInt64LE(offset);
  return { magic, version, bump, authority, snapshotHash, totalActivePower, windowStartEpoch, windowEndEpoch, createdSlot, expiresAt, createdAt };
}

export function decodeCitizenPower(data: Buffer): DecodedCitizenPower {
  assertLen(data, CITIZEN_POWER_LEN, "CitizenPower");
  let offset = 0;
  const magic = bytes(data, offset, 8).toString("utf8"); offset += 8;
  if (magic !== CITIZEN_POWER_MAGIC) throw new Error(`Invalid CitizenPower magic: ${magic}`);
  const version = data.readUInt16LE(offset); offset += 2;
  const bump = data.readUInt8(offset++);
  const powerSnapshot = new PublicKey(bytes(data, offset, 32)); offset += 32;
  const citizen = new PublicKey(bytes(data, offset, 32)); offset += 32;
  const snapshotHash = bytes(data, offset, 32); offset += 32;
  const power = data.readBigUInt64LE(offset); offset += 8;
  const settledSlot = data.readBigUInt64LE(offset); offset += 8;
  const settledAt = data.readBigInt64LE(offset); offset += 8;
  const expiresAt = data.readBigInt64LE(offset);
  return { magic, version, bump, powerSnapshot, citizen, snapshotHash, power, settledSlot, settledAt, expiresAt };
}

export function civilizationRequiredPower(totalActivePower: bigint | number, thresholdBps: number): bigint {
  const total = BigInt(totalActivePower);
  const bps = BigInt(thresholdBps);
  if (total <= 0n || bps <= 0n || bps > 10_000n) {
    throw new Error("Invalid civilization threshold inputs");
  }
  return (total * bps + 9_999n) / 10_000n;
}

export function civilizationThresholdMet({
  yesPower,
  noPower = 0,
  requiredPower,
}: {
  yesPower: bigint | number;
  noPower?: bigint | number;
  requiredPower: bigint | number;
}): boolean {
  const yes = BigInt(yesPower);
  const no = BigInt(noPower);
  const required = BigInt(requiredPower);
  return yes >= required && yes > no;
}

export function hash32(value: Buffer | Uint8Array | string): Buffer {
  if (typeof value === "string") {
    const normalized = value.startsWith("sha256:") ? value.slice(7) : value;
    if (!/^[0-9a-fA-F]{64}$/.test(normalized)) {
      throw new Error("Expected a 32-byte hex string or sha256:<hex> hash");
    }
    return Buffer.from(normalized, "hex");
  }
  const buffer = Buffer.from(value);
  if (buffer.length !== 32) throw new Error(`Expected 32 hash bytes, got ${buffer.length}`);
  return buffer;
}

function bytes(data: Buffer, offset: number, length: number): Buffer {
  return Buffer.from(data.subarray(offset, offset + length));
}

function assertLen(data: Buffer, expected: number, name: string): void {
  if (data.length !== expected) {
    throw new Error(`Invalid ${name} length: expected ${expected}, got ${data.length}`);
  }
}
