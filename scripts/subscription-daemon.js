#!/usr/bin/env node
/**
 * FluxaPay Subscription Indexer Daemon
 *
 * Polls the chain for due subscriptions and calls process_due_subscriptions
 * on the RefundManager contract. Designed to run as a cron job or long-running
 * process.
 *
 * Usage:
 *   node scripts/subscription-daemon.js
 *
 * Environment variables (see .env.example):
 *   STELLAR_RPC_URL        – Soroban RPC endpoint
 *   CONTRACT_ID            – RefundManager contract address
 *   OPERATOR_SECRET        – Operator secret key (settlement_operator role)
 *   POLL_INTERVAL_MS       – How often to poll in ms (default: 60000)
 *   NETWORK_PASSPHRASE     – Stellar network passphrase
 */

"use strict";

const {
  SorobanRpc,
  TransactionBuilder,
  Networks,
  Keypair,
  Contract,
  nativeToScVal,
  BASE_FEE,
  xdr,
} = require("@stellar/stellar-sdk");

const RPC_URL = process.env.STELLAR_RPC_URL || "https://soroban-testnet.stellar.org";
const CONTRACT_ID = process.env.CONTRACT_ID;
const OPERATOR_SECRET = process.env.OPERATOR_SECRET;
const POLL_INTERVAL_MS = parseInt(process.env.POLL_INTERVAL_MS || "60000", 10);
const NETWORK_PASSPHRASE = process.env.NETWORK_PASSPHRASE || Networks.TESTNET;

if (!CONTRACT_ID || !OPERATOR_SECRET) {
  console.error("ERROR: CONTRACT_ID and OPERATOR_SECRET must be set.");
  process.exit(1);
}

const server = new SorobanRpc.Server(RPC_URL, { allowHttp: RPC_URL.startsWith("http://") });
const operatorKeypair = Keypair.fromSecret(OPERATOR_SECRET);
const contract = new Contract(CONTRACT_ID);

/**
 * Fetch all active subscription IDs from contract storage by scanning
 * the SubscriptionCounter and individual Subscription entries.
 *
 * In production this would be replaced by a Mercury/Horizon event query
 * that tracks SUBSCRIPTION/CREATED events to build the index off-chain.
 */
async function fetchDueSubscriptionIds() {
  // Query the SubscriptionCounter to know the range of IDs to check.
  const counterKey = xdr.LedgerKey.contractData(
    new xdr.LedgerKeyContractData({
      contract: contract.address().toScAddress(),
      key: xdr.ScVal.scvLedgerKeyContractInstance(),
      durability: xdr.ContractDataDurability.persistent(),
    })
  );

  // Fetch ledger entries for subscriptions via RPC getLedgerEntries.
  // The daemon relies on an off-chain index (e.g. PostgreSQL populated by
  // the Mercury indexer sync.yml) to enumerate subscription IDs efficiently.
  // Here we read from a local JSON file written by the indexer as a fallback.
  const fs = require("fs");
  const indexPath = process.env.SUBSCRIPTION_INDEX_PATH || "/tmp/fluxapay_subscriptions.json";

  if (!fs.existsSync(indexPath)) {
    console.warn(`[daemon] No subscription index found at ${indexPath}. Skipping cycle.`);
    return [];
  }

  const index = JSON.parse(fs.readFileSync(indexPath, "utf8"));
  const nowSecs = Math.floor(Date.now() / 1000);

  // Filter to subscriptions that are Active and past their next_payment_at.
  return (index.subscriptions || [])
    .filter(
      (s) =>
        s.status === "Active" &&
        (s.next_payment_at <= nowSecs || (s.next_retry_at && s.next_retry_at <= nowSecs))
    )
    .map((s) => s.subscription_id);
}

/**
 * Call process_due_subscriptions on the contract.
 * Returns the number of subscriptions processed.
 */
async function triggerProcessDue() {
  const account = await server.getAccount(operatorKeypair.publicKey());

  const tx = new TransactionBuilder(account, {
    fee: BASE_FEE,
    networkPassphrase: NETWORK_PASSPHRASE,
  })
    .addOperation(
      contract.call(
        "process_due_subscriptions",
        nativeToScVal(operatorKeypair.publicKey(), { type: "address" })
      )
    )
    .setTimeout(30)
    .build();

  const preparedTx = await server.prepareTransaction(tx);
  preparedTx.sign(operatorKeypair);

  const sendResult = await server.sendTransaction(preparedTx);
  if (sendResult.status === "ERROR") {
    throw new Error(`Transaction failed: ${JSON.stringify(sendResult.errorResult)}`);
  }

  // Poll for confirmation.
  let getResult;
  for (let i = 0; i < 10; i++) {
    await sleep(3000);
    getResult = await server.getTransaction(sendResult.hash);
    if (getResult.status !== "NOT_FOUND") break;
  }

  if (getResult.status === "SUCCESS") {
    const processed = getResult.returnValue
      ? parseInt(getResult.returnValue.value(), 10)
      : 0;
    return processed;
  }

  throw new Error(`Transaction did not succeed: ${getResult.status}`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function runCycle() {
  console.log(`[daemon] ${new Date().toISOString()} – starting cycle`);
  try {
    const dueIds = await fetchDueSubscriptionIds();
    if (dueIds.length === 0) {
      console.log("[daemon] No due subscriptions found.");
      return;
    }
    console.log(`[daemon] Found ${dueIds.length} due subscription(s). Triggering contract call…`);
    const processed = await triggerProcessDue();
    console.log(`[daemon] process_due_subscriptions returned: ${processed} processed.`);
  } catch (err) {
    console.error("[daemon] Cycle error:", err.message || err);
  }
}

async function main() {
  console.log(`[daemon] Starting FluxaPay subscription daemon (poll every ${POLL_INTERVAL_MS}ms)`);
  console.log(`[daemon] Contract: ${CONTRACT_ID}`);
  console.log(`[daemon] Operator: ${operatorKeypair.publicKey()}`);

  // Run immediately, then on interval.
  await runCycle();
  setInterval(runCycle, POLL_INTERVAL_MS);
}

main().catch((err) => {
  console.error("[daemon] Fatal:", err);
  process.exit(1);
});
