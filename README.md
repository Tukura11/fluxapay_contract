Fluxapay is a payment gateway on the Stellar blockchain that enables merchants to accept crypto payments and get settled in their local fiat currency.

FluxaPay bridges the gap between crypto payments and real-world commerce—making stablecoin payments as easy to integrate as Stripe.

## CI/CD

[![CI](https://github.com/MetroLogic/fluxapay_contract/actions/workflows/ci.yml/badge.svg)](https://github.com/MetroLogic/fluxapay_contract/actions/workflows/ci.yml)
[![CD](https://github.com/MetroLogic/fluxapay_contract/actions/workflows/cd.yml/badge.svg)](https://github.com/MetroLogic/fluxapay_contract/actions/workflows/cd.yml)
Automated testing and deployment pipeline using GitHub Actions:

- **CI:** Runs tests, linting, and builds on every push/PR to main
- **CD:** Auto-deploys to development and staging on merge to main; production requires manual approval
- All tests must pass before deployment

---

## What Problem does Fluxapay solve?

Despite growing crypto adoption, everyday commerce remains largely fiat-based.

A major pain point is that crypto-native customers are forced to offramp every time they want to pay a merchant. This introduces:

•⁠ ⁠Extra fees from offramping and FX conversions  
•⁠ ⁠Payment delays and failed transactions  
•⁠ ⁠Poor checkout experience for crypto users  
•⁠ ⁠Lost sales for merchants

At the same time, merchants want to accept crypto without holding volatile assets, managing wallets, or dealing with on-chain complexity.

Fluxapay solves this by enabling _USDC-in → fiat-out_ payments with a merchant-friendly experience.

## How FluxaPay Works

1.⁠ ⁠*Merchant Creates a Charge*  
 Merchant creates a payment request via API or Payment Link.

2.⁠ ⁠*Customer Pays in USDC (Stellar)*  
 Customer pays from any supported Stellar wallet.

3.⁠ ⁠*Instant Verification*  
 FluxaPay verifies the payment on-chain and updates the payment status in real-time.

4.⁠ ⁠*Settlement to Merchant (Local Fiat)*  
 FluxaPay converts and settles the value to the merchant’s preferred local currency via bank transfer or supported payout channels.

## Key Features

### Developer Platform (Stripe-like)

•⁠ ⁠*Merchant API for Seamless Integration*

- Create payments/charges
- Fetch payment status
- Issue refunds (where supported)
- Manage customers & metadata
  •⁠ ⁠*Webhooks*
- ⁠ payment.created ⁠, ⁠ payment.pending ⁠, ⁠ payment.confirmed ⁠, ⁠ payment.failed ⁠, ⁠ payment.settled ⁠

### No-Code / Low-Code

•⁠ ⁠*Payment Links*

- Shareable links for quick checkout (social commerce, WhatsApp, Instagram, etc.)
  •⁠ ⁠*Invoices*
- Generate invoices with payment links and track payment status
- Perfect for freelancers, agencies, and B2B billing

### Merchant Tools

•⁠ ⁠Merchant Dashboard & Analytics
•⁠ ⁠Reconciliation Reports
•⁠ ⁠Built for Emerging Markets

## Typical Integrations

### 1) Checkout on your website/app

•⁠ ⁠Merchant calls FluxaPay API to create a payment
•⁠ ⁠Customer completes payment via hosted checkout or embedded flow
•⁠ ⁠Fluxapay sends webhook when confirmed
•⁠ ⁠Merchant fulfills the order

### 2) Payment links for invoices & social commerce

•⁠ ⁠Merchant generates a payment link (amount, currency, description)
•⁠ ⁠Customer pays using Stellar USDC
•⁠ ⁠Merchant is notified via dashboard + webhook/email (optional)

## Tech Stack (Planned)

•⁠ ⁠*Blockchain:* Stellar  
•⁠ ⁠*Stablecoin Rail:* USDC on Stellar  
•⁠ ⁠*Backend:* Node.js (TBD)  
•⁠ ⁠*Smart Contracts:* Stellar Soroban
•⁠ ⁠*Database:* PostgreSQL  
•⁠ ⁠*APIs:* REST + Webhooks  
•⁠ ⁠*Frontend:* Next.js (Merchant Dashboard)  
•⁠ ⁠*FX & Settlement:* On-chain liquidity + payout partners

## Use Cases

•⁠ ⁠E-commerce stores and marketplaces
•⁠ ⁠SaaS and subscription businesses
•⁠ ⁠Freelancers & agencies (invoices + payment links)
•⁠ ⁠Cross-border payments for global customers
•⁠ ⁠Merchants in emerging markets accepting stablecoin payments

## Vision

Make stablecoin payments simple, practical, and accessible so merchants can sell globally while customers pay directly with USDC, without offramping friction.

## Roadmap

•⁠ ⁠[ ] Core payment gateway (USDC on Stellar)
•⁠ ⁠[ ] Merchant dashboard
•⁠ ⁠[ ] API for payments + webhooks
•⁠ ⁠[ ] Payment links
•⁠ ⁠[ ] Invoicing
•⁠ ⁠[ ] SDKs
•⁠ ⁠[ ] Fiat settlement integrations
•⁠ ⁠[ ] Refunds & dispute tooling (where applicable)
•⁠ ⁠[ ] Multi-currency support & expanded stablecoins

## Contributing

Contributions are welcome!  
Open an issue or submit a PR to help build Fluxapay.

## Security

Please refer to our [Security Policy](SECURITY.md) for information on reporting vulnerabilities and our current audit status.

## Telegram link

<https://t.me/+m23gN14007w0ZmQ0>
