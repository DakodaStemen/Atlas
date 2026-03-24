---
name: compliance-healthcare-fintech
description: Healthcare (HIPAA, PHI de-identification, medical device IoT) and fintech (PCI-DSS, payment processing) compliance patterns. Use when building systems that handle PHI, medical device data, cardholder data, or payment processing.
domain: compliance
tags: [hipaa, phi, pci-dss, fintech, payments, healthcare, iot, medical-device, compliance]
triggers: HIPAA, PHI, de-identification, safe harbor, medical device, IoMT, PCI-DSS, cardholder data, payment processing, Stripe, SCA, PSD2
---

# Healthcare & Fintech Compliance

## HIPAA Compliance

### Data Protection

- **At Rest**: AES-256 for all databases, file storage, and backups.
- **In Transit**: TLS 1.2+ for all network traffic. Disable weak ciphers and legacy SSL.
- **Key Management**: Dedicated KMS/HSM; rotate keys annually.

### Immutable Audit Logging

- Log: User ID, timestamp, action (View/Edit/Delete), specific record ID.
- Store on separate, write-once-read-many (WORM) storage or signed to prevent tampering.
- Retention: HIPAA requires documentation retained for at least **6 years**.

### Minimum Necessary Access

- Strict RBAC with role-per-function granularity.
- Automatic logoff: session timeouts (15 minutes) for unattended workstations.
- Unique identifiers: every person with PHI access must have a unique login. No shared accounts.

### Business Associate Agreements (BAA)

- Sign BAA with every third-party service that touches PHI (AWS, GCP, Twilio, etc.).
- Verify vendors are HIPAA-eligible and configured in "HIPAA mode."

### Developer Implementation Guide

- **Database**: Encrypt PHI columns with application-level encryption; use field-level encryption for highest sensitivity.
- **API Design**: Never return PHI in error messages or logs; use opaque reference IDs, not SSN or MRN in URLs.
- **Search**: Implement tokenized or hashed search for PHI fields rather than plaintext search indexes.
- **Caching**: Never cache PHI in browser localStorage or sessionStorage; server-side cache must be encrypted and access-controlled.
- **Testing**: Use synthetic data generators for test environments; never copy production PHI to dev/staging.

### Emergency Access (Break-Glass)

- Pre-provisioned elevated-access accounts for emergencies.
- Every action under break-glass logged with enhanced detail and reviewed post-incident within 24 hours.

---

## PHI De-Identification (Safe Harbor)

### The 18 HIPAA Identifiers to Remove or Generalize

1. Names
2. Geographic data (below state level)
3. Dates (except year) related to an individual
4. Phone numbers
5. Fax numbers
6. Email addresses
7. Social Security numbers
8. Medical record numbers
9. Health plan beneficiary numbers
10. Account numbers
11. Certificate/license numbers
12. Vehicle identifiers and serial numbers
13. Device identifiers and serial numbers
14. Web URLs
15. IP addresses
16. Biometric identifiers
17. Full-face photographs
18. Any other unique identifying number

### Safe Harbor Method

Remove all 18 identifiers AND certify no residual information can identify an individual. Date generalization: truncate to year; geographic: truncate ZIP to first 3 digits (or 000 if population < 20,000).

### Expert Determination Method

A qualified statistical expert certifies the risk of re-identification is "very small." Must document the methods and results of the analysis.

---

## Medical Device Integration (IoMT)

### Architecture Patterns

- **Edge Gateway Model**: Devices connect to a local gateway that handles protocol translation, buffering, and secure forwarding to cloud.
- **Direct Cloud**: Only for devices with robust TLS stacks and reliable connectivity.
- **Hybrid**: Critical alerts go direct; bulk data goes through gateway.

### Security Requirements

- Device authentication via X.509 certificates or pre-shared keys.
- All data encrypted in transit (TLS 1.2+ or DTLS for UDP).
- Firmware signing and secure boot chain.
- Network segmentation: medical devices on isolated VLAN.

### Data Integrity

- Timestamping at device level with NTP synchronization.
- Checksums or HMAC on transmitted data.
- Audit trail from device to storage for regulatory traceability.

---

## PCI-DSS Compliance

### Core Requirements

1. **Network Security**: Install and maintain firewalls; do not use vendor defaults for system passwords.
2. **Data Protection**: Protect stored cardholder data (CHD); encrypt CHD transmission over open networks.
3. **Vulnerability Management**: Use and update antivirus; develop and maintain secure systems.
4. **Access Control**: Restrict access on need-to-know; assign unique ID per user; restrict physical access.
5. **Monitoring**: Track and monitor all access to network resources and CHD.
6. **Policy**: Maintain information security policy for all personnel.

### PCI-DSS Level 1 (6M+ Annual Transactions)

- Annual on-site audit by QSA (Qualified Security Assessor).
- Quarterly ASV (Approved Scanning Vendor) network scans.
- Annual penetration test (internal and external).
- Segmentation testing every 6 months.
- File Integrity Monitoring (FIM) on critical system files.
- Centralized log management with 1-year retention (3 months immediately available).

### Cardholder Data Handling

- **Masking**: Display only last 4 digits; never log full PAN.
- **Tokenization**: Replace PAN with non-reversible token for storage.
- **Encryption**: AES-256 for stored CHD; TLS 1.2+ in transit.
- **SAD (Sensitive Auth Data)**: Never store CVV, PIN, or full track data post-authorization.

---

## Fintech Payments Patterns

### Stripe Integration

- Use Stripe Elements or Checkout for PCI SAQ-A compliance (card data never touches your server).
- Implement idempotency keys on all POST requests to prevent duplicate charges.
- Webhook signature verification: always validate `Stripe-Signature` header using HMAC.

### Webhook Reliability

- Verify signatures before processing; reject unsigned or expired events.
- Implement idempotent event handlers (process each event ID at most once).
- Return 200 quickly; do heavy processing asynchronously.
- Handle out-of-order events (check object state, not event sequence).

### Financial Reconciliation

- Daily automated reconciliation between Stripe dashboard and internal records.
- Track every state transition: charge created, captured, refunded, disputed.
- Maintain immutable audit trail of all financial transactions.

### PSD2/SCA Compliance

- Strong Customer Authentication (SCA) required for European electronic payments.
- Use Stripe Payment Intents API with `confirmation_method: 'manual'` for SCA flows.
- Implement 3D Secure 2 for card authentication.
- Handle SCA exemptions (low-value, trusted beneficiary, recurring) per regulation.
