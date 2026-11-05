# Intent Capture & Framing Questions

## Sources

- [desc] Initial description: "BRD-001-product-vision-scope-and-success.md"
- [scope] Workflow-selected scope: `classic`.
- [memory:M1] `aidlc/spaces/default/memory/org.md#Engineering Standards`: "Every requirement has a testable acceptance criterion."
- [memory:M2] `aidlc/spaces/default/memory/org.md#Walking Skeleton`: "Scope-dependent. Greenfield work starts with the thinnest end-to-end slice."
- [memory:M3] `aidlc/spaces/default/memory/org.md#Testing Posture`: "Standard: requirement-driven tests with a happy-path floor per component."
- [memory:M4] `aidlc/spaces/default/memory/team.md#Team Practices`: "Trunk-based development with short-lived branches."

The referenced BRD-001 file is input context only. Its claims reach the artifacts only through the confirmed answers below.

## Q1. What business problem is this initiative solving, and who is the customer who feels that pain?

A. Retail crypto investors cannot confidently answer "what do I own, what is it worth, and am I up or down?" across exchanges, wallets, and chains. The product is a read-only aggregation and decision-support layer with user-initiated order intents, for mainstream retail (primary) and crypto-native active investors (secondary); family offices are post-MVP.
B. The customer is crypto-native active investors only; mainstream retail simplicity is out of scope.
C. The customer is internal operations staff reconciling portfolios; no consumer-facing product.
D. The problem is market-data aggregation only; portfolio valuation and P&L are out of scope.
E. Not yet defined.
X. Other (describe)

[Answer]: A. Retail crypto investors cannot confidently answer "what do I own, what is it worth, and am I up or down?" across exchanges, wallets, and chains. The product is a read-only aggregation and decision-support layer with user-initiated order intents, for mainstream retail (primary) and crypto-native active investors (secondary); family offices are post-MVP.

## Q2. What does success look like, and which metrics decide whether this initiative succeeded?

A. Use BRD-001's success metrics: valuation accuracy >99% vs manual calculation; price freshness ≤5 minutes for top-100 assets and ≤15 minutes for the tail; activation ≥40% of sign-ups reach a first valuation within 24 hours; D30 retention ≥25%; free-to-paid conversion ≥3%; ≥99.9% monthly availability for paid plans.
B. Valuation accuracy and data freshness only; retention and conversion are validated later.
C. Activation within 24 hours and D30 retention only.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. Use BRD-001's success metrics: valuation accuracy >99% vs manual calculation; price freshness ≤5 minutes for top-100 assets and ≤15 minutes for the tail; activation ≥40% of sign-ups reach a first valuation within 24 hours; D30 retention ≥25%; free-to-paid conversion ≥3%; ≥99.9% monthly availability for paid plans.

## Q3. What triggered this initiative, who decides scope and priority, and what communication cadence is required?

A. Trigger: a global mass-market retail launch, with a 2026-11-04 readiness review closing scope and compliance gaps. The product owner decides scope and priority; engineering, security, and legal leads influence. Written status is reported at each workflow stage gate, with decisions recorded in the repository.
B. Trigger: technical debt cleanup; the engineering lead decides scope; ad-hoc communication.
C. Trigger: a regulatory deadline; legal decides scope; formal monthly reporting.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. Trigger: a global mass-market retail launch, with a 2026-11-04 readiness review closing scope and compliance gaps. The product owner decides scope and priority; engineering, security, and legal leads influence. Written status is reported at each workflow stage gate, with decisions recorded in the repository.

## Q4. What must this first version explicitly NOT do?

A. Never custody assets or private keys; never place trades automatically (a human approves every user-initiated order intent); no fiat on/off-ramp; no tax filing or regulated advice; no futures or margin; the product initiates no transfers. Direct trade execution, DeFi/staking/LP accounting, NFT valuation, native mobile apps, multi-user/advisor portals, and KYC/AML are post-MVP, not part of this first version.
B. Only exclude custody of private keys; automatic trade execution may be added.
C. Only exclude tax filing and regulated advice; everything else is in scope.
D. Not yet defined.
E. None.
X. Other (describe)

[Answer]: A. Never custody assets or private keys; never place trades automatically (a human approves every user-initiated order intent); no fiat on/off-ramp; no tax filing or regulated advice; no futures or margin; the product initiates no transfers. Direct trade execution, DeFi/staking/LP accounting, NFT valuation, native mobile apps, multi-user/advisor portals, and KYC/AML are post-MVP, not part of this first version.

## Assumption Confirmation

The stakeholder map records one assumption: customer communication channels and cadence are not yet defined.

A. Accept assumptions
B. Convert to follow-up questions

[Answer]: A. Accept assumptions

