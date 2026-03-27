# Security Policy

FluxaPay takes the security of our smart contracts and user funds (USDC) extremely seriously. This document outlines our vulnerability disclosure policy and current audit status.

## 🛡️ Vulnerability Disclosure Policy

If you discover a security vulnerability, we encourage you to report it to us responsibly. We will acknowledge receipt of your report and provide a status update as we investigate and address the issue.

### Reporting a Vulnerability

Please send an email to: **security@metrologic.com**

To help us prioritize and address your report, please include:

- A detailed description of the vulnerability.
- Steps to reproduce the issue (PoC code or clear instructions).
- Your assessment of the impact.

### Response SLA

- **Acknowledgment**: Within 48 hours of receipt.
- **Resolution**: Varies depending on severity; we aim for rapid patches of critical issues.

### Scope

- **In-Scope**: Core Soroban smart contracts (`fluxapay/src/*.rs`).
- **Out-of-Scope**: Third-party protocols (Stellar/Soroban platform), front-end interfaces (unless they impact contract security).

## 💰 Bug Bounty Program

A public bug bounty program is currently **in development**. Until then, we may provide discretionary rewards for high-impact, responsibly disclosed vulnerabilities.

## 🔍 Audit Status

| Audit Date | Auditor  | Scope           | Status               | Report Link        |
| ---------- | -------- | --------------- | -------------------- | ------------------ |
| 2026-03-27 | Internal | All Contracts   | Completed (Internal) | N/A                |
| TBD        | External | Mainnet Release | Upcoming             | [Link Placeholder] |

> [!IMPORTANT]
> This project is currently in **active development**. Use with caution and only on Testnet for now.

## 🔐 Code Ownership

Security-critical files, such as `access_control.rs` and the main `lib.rs` payment logic, require mandatory review from the security team as defined in our [`CODEOWNERS`](.github/CODEOWNERS) file.
