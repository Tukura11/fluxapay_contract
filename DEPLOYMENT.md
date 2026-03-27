# Deployment and Operations Guide

This document records the deployed contract addresses, network configurations, and operational procedures for the FluxaPay protocol on the Stellar network.

## 🚀 Contract Registry

| Contract Name    | Network | Contract ID              | Deploy Date  | Deployer Address     |
| ---------------- | ------- | ------------------------ | ------------ | -------------------- |
| PaymentProcessor | Testnet | `<PAYMENT_PROCESSOR_ID>` | `YYYY-MM-DD` | `<DEPLOYER_ADDRESS>` |
| RefundManager    | Testnet | `<REFUND_MANAGER_ID>`    | `YYYY-MM-DD` | `<DEPLOYER_ADDRESS>` |
| MerchantRegistry | Testnet | `<MERCHANT_REGISTRY_ID>` | `YYYY-MM-DD` | `<DEPLOYER_ADDRESS>` |
| FXOracle         | Testnet | `<FX_ORACLE_ID>`         | `YYYY-MM-DD` | `<DEPLOYER_ADDRESS>` |

> [!NOTE]
> For Mainnet addresses, please refer to the secure internal dashboard or contact the operations lead.

## 🌐 Network Configuration

### Stellar Testnet

- **Horizon URL**: `https://horizon-testnet.stellar.org`
- **Network Passphrase**: `Test SDF Network ; September 2015`
- **RPC URL**: `https://soroban-testnet.stellar.org`

### Stellar Mainnet

- **Horizon URL**: `https://horizon.stellar.org`
- **Network Passphrase**: `Public Global Stellar Network ; September 2015`
- **RPC URL**: `https://soroban-rpc.stellar.org`

## 🛠 Upgrade Process

The FluxaPay contracts follow the standard Soroban upgrade pattern. Upgrading a contract requires administrative authorization.

### Step 1: Upload New WASM

First, install the new WASM code on the network without deploying it to an instance.

```bash
stellar contract install --wasm target/wasm32-unknown-unknown/release/fluxapay.wasm --network testnet --source <ADMIN_SECRET>
```

Take note of the returned `Wasm Hash`.

### Step 2: Invoke Upgrade

Invoke the `upgrade` function (if implemented) or use the `stellar-cli` to update the contract instance's executable.

> [!IMPORTANT]
> If a custom `upgrade` function is not present in the contract logic, a logic update or a redeployment/migration may be required depending on the specific contract state management.

```bash
# Example if using a custom upgrade function (recommended for state migration)
stellar contract invoke --id <CONTRACT_ID> --network testnet --source <ADMIN_SECRET> -- upgrade --new_wasm_hash <WASM_HASH>
```

## 🔑 Admin Key Rotation

Administrative roles for `PaymentProcessor` and `RefundManager` are managed via the `AccessControl` module.

### Rotating the Admin Role

To transfer administrative control to a new address:

```bash
stellar contract invoke --id <CONTRACT_ID> --network testnet --source <CURRENT_ADMIN_SECRET> \
  -- transfer_admin --current_admin <CURRENT_ADMIN_ADDRESS> --new_admin <NEW_ADMIN_ADDRESS>
```

> [!WARNING]
> Ensure the new admin address is correct and accessible before performing this operation. Administrative control can only be transferred, not recovered without the current admin's signature.

## ✅ On-Chain Verification

To verify the current state and administrative configuration of a deployed contract:

### Verify Admin

```bash
stellar contract invoke --id <CONTRACT_ID> --network testnet -- get_admin
```

### Verify Registry Info

```bash
# Verify a merchant in the registry
stellar contract invoke --id <MERCHANT_REGISTRY_ID> --network testnet -- get_merchant --merchant_id <MERCHANT_ADDRESS>
```

### Health Check (Simulation)

```bash
# Check if the contract is responsive
stellar contract info interface --id <CONTRACT_ID> --network testnet
```
