# Contract Summary — Crypto Pocket Butler

Every boundary in the unit DAG (`units-generation/unit-of-work-dependency.md`) between the units
defined in `units-generation/unit-of-work.md`, specified as OpenAPI 3.1 in YAML, versioned in the
URL path (`/v1/...`), with RFC 7807 `application/problem+json` errors carrying a `correlationId`.
The gateway API is internal and documented for the SPA and Support Console only.

## Contracts

| # | Provider Unit | Consumer | Mechanism | Owner |
|---|---|---|---|---|
| C1 | U1 ApiGateway | U2 WebApp (internal) | HTTP/JSON, OpenAPI 3.1 | U1 |
| C2 | U1 ApiGateway | U13 Support Console (internal) | HTTP/JSON, OpenAPI 3.1 | U1 |
| C3 | U3 Identity | U1 ApiGateway | HTTP/JSON, OpenAPI 3.1 | U3 |
| C4 | U1 Asset Registry | U4, U6, U7 | HTTP/JSON, OpenAPI 3.1 | U1 |
| C5 | U4 Account | U6, U7, U11 | HTTP/JSON, OpenAPI 3.1 | U4 |
| C6 | U5 Market Data | U6, U8, U11 | HTTP/JSON, OpenAPI 3.1 | U5 |
| C7 | U7 Ledger | U6, U8 | HTTP/JSON, OpenAPI 3.1 | U7 |
| C8 | U6 Portfolio | U8 | HTTP/JSON, OpenAPI 3.1 | U6 |
| C9 | U9 Notification | U4, U6, U8 | HTTP/JSON, OpenAPI 3.1 | U9 |
| C10 | U12 Audit | U3, U4, U5, U6, U7, U8, U9, U10, U11 | HTTP/JSON, OpenAPI 3.1 | U12 |
| C11 | Each service | U11 Job Orchestration | HTTP/JSON run endpoint, shared service token | Each provider |
| C12 | U10 Analytics | U1 ApiGateway | HTTP/JSON, OpenAPI 3.1 | U10 |
| C13 | Supabase Auth | U2 WebApp | External OIDC/PKCE | Supabase (external) |
| C14 | WalletConnect Relay | U2 WebApp | External SDK protocol | WalletConnect (external) |

## C1 — Gateway API (U1 → U2)

```yaml
openapi: 3.1.0
info:
  title: Crypto Pocket Butler Gateway API
  version: 1.0.0
servers:
  - url: https://api.cryptopocketbutler.example/v1
security:
  - supabaseJwt: []
paths:
  /portfolios:
    get:
      operationId: listPortfolios
      responses:
        "200":
          description: Portfolios owned by the caller
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/Portfolio" }
        "401": { $ref: "#/components/responses/Unauthorized" }
  /portfolios/{portfolioId}:
    get:
      operationId: getPortfolio
      parameters:
        - name: portfolioId
          in: path
          required: true
          schema: { type: string, format: uuid }
      responses:
        "200":
          description: Portfolio valuation
          content:
            application/json:
              schema: { $ref: "#/components/schemas/PortfolioValuation" }
        "403": { $ref: "#/components/responses/Forbidden" }
        "404": { $ref: "#/components/responses/NotFound" }
  /accounts:
    post:
      operationId: addAccount
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: "#/components/schemas/AddAccountRequest" }
      responses:
        "201":
          description: Account created
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Account" }
        "422": { $ref: "#/components/responses/Problem" }
  /portfolios/{portfolioId}/order-intents:
    post:
      operationId: createOrderIntent
      parameters:
        - name: portfolioId
          in: path
          required: true
          schema: { type: string, format: uuid }
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: "#/components/schemas/OrderIntentRequest" }
      responses:
        "201":
          description: Intent recorded; allocation updates immediately
          content:
            application/json:
              schema: { $ref: "#/components/schemas/OrderIntent" }
        "409": { $ref: "#/components/responses/Problem" }
components:
  securitySchemes:
    supabaseJwt:
      type: http
      scheme: bearer
      bearerFormat: JWT
  responses:
    Problem:
      description: RFC 7807 problem
      content:
        application/problem+json:
          schema: { $ref: "#/components/schemas/Problem" }
    Unauthorized:
      description: Missing or invalid token
      content:
        application/problem+json:
          schema: { $ref: "#/components/schemas/Problem" }
    Forbidden:
      description: Not the owner or not permitted
      content:
        application/problem+json:
          schema: { $ref: "#/components/schemas/Problem" }
    NotFound:
      description: Not found or not visible to the caller
      content:
        application/problem+json:
          schema: { $ref: "#/components/schemas/Problem" }
  schemas:
    Problem:
      type: object
      required: [type, title, status, correlationId]
      properties:
        type: { type: string }
        title: { type: string }
        status: { type: integer }
        detail: { type: string }
        correlationId: { type: string }
    Portfolio:
      type: object
      required: [id, name, baseCurrency]
      properties:
        id: { type: string, format: uuid }
        name: { type: string }
        baseCurrency: { type: string, example: USD }
    PortfolioValuation:
      type: object
      required: [portfolioId, asOf, totalValue, currency, pricedCoverage, unpricedCount]
      properties:
        portfolioId: { type: string, format: uuid }
        asOf: { type: string, format: date-time }
        totalValue: { type: string, description: decimal string }
        currency: { type: string }
        pricedCoverage: { type: number }
        unpricedCount: { type: integer }
    AddAccountRequest:
      type: object
      required: [kind, label]
      properties:
        kind: { type: string, enum: [exchange, wallet, csv] }
        venue: { type: string, enum: [binance, okx, evm, bnb, solana] }
        label: { type: string }
        address: { type: string }
    Account:
      type: object
      required: [id, kind, label, status]
      properties:
        id: { type: string, format: uuid }
        kind: { type: string }
        label: { type: string }
        status: { type: string, enum: [syncing, synced, failed] }
        lastSyncedAt: { type: string, format: date-time }
    OrderIntentRequest:
      type: object
      required: [assetId, side, quantity]
      properties:
        assetId: { type: string }
        side: { type: string, enum: [buy, sell] }
        quantity: { type: string, description: decimal string }
    OrderIntent:
      type: object
      required: [id, assetId, side, quantity, status]
      properties:
        id: { type: string, format: uuid }
        assetId: { type: string }
        side: { type: string, enum: [buy, sell] }
        quantity: { type: string }
        status: { type: string, enum: [pending, cancelled, executed] }
```

## C2 — Support API (U1 → U13)

```yaml
openapi: 3.1.0
info:
  title: Crypto Pocket Butler Support API
  version: 1.0.0
servers:
  - url: https://api.cryptopocketbutler.example/v1/support
security:
  - supabaseJwt: []
paths:
  /accounts/{accountId}/metadata:
    get:
      operationId: getAccountMetadata
      description: Account metadata only; never credentials or portfolio contents.
      parameters:
        - name: accountId
          in: path
          required: true
          schema: { type: string, format: uuid }
        - name: reason
          in: query
          required: true
          schema: { type: string }
      responses:
        "200":
          description: Metadata
          content:
            application/json:
              schema: { $ref: "#/components/schemas/AccountMetadata" }
        "403": { $ref: "#/components/responses/Forbidden" }
  /access-grants:
    post:
      operationId: requestSupportAccess
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: "#/components/schemas/SupportAccessRequest" }
      responses:
        "201":
          description: Time-bound grant created; a second approver is required
          content:
            application/json:
              schema: { $ref: "#/components/schemas/SupportAccessGrant" }
components:
  securitySchemes:
    supabaseJwt:
      type: http
      scheme: bearer
      bearerFormat: JWT
  responses:
    Forbidden:
      description: Not permitted
      content:
        application/problem+json:
          schema: { $ref: "#/components/schemas/Problem" }
  schemas:
    Problem:
      type: object
      required: [type, title, status, correlationId]
      properties:
        type: { type: string }
        title: { type: string }
        status: { type: integer }
        correlationId: { type: string }
    AccountMetadata:
      type: object
      required: [id, kind, label, status]
      properties:
        id: { type: string, format: uuid }
        kind: { type: string }
        label: { type: string }
        status: { type: string }
        lastSyncedAt: { type: string, format: date-time }
    SupportAccessRequest:
      type: object
      required: [userId, reason]
      properties:
        userId: { type: string, format: uuid }
        reason: { type: string }
    SupportAccessGrant:
      type: object
      required: [id, userId, reason, expiresAt, status]
      properties:
        id: { type: string, format: uuid }
        userId: { type: string, format: uuid }
        reason: { type: string }
        expiresAt: { type: string, format: date-time }
        status: { type: string, enum: [pending_approval, active, expired] }
```

## C3 — Identity internal API (U3 → U1)

```yaml
openapi: 3.1.0
info:
  title: Identity Internal API
  version: 1.0.0
paths:
  /v1/principals/{authUserId}:
    get:
      operationId: resolvePrincipal
      parameters:
        - name: authUserId
          in: path
          required: true
          schema: { type: string, format: uuid }
      responses:
        "200":
          description: Principal and role
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Principal" }
        "404": { $ref: "#/components/responses/NotFound" }
  /v1/consent/{userId}:
    get:
      operationId: getConsent
      parameters:
        - name: userId
          in: path
          required: true
          schema: { type: string, format: uuid }
      responses:
        "200":
          description: Consent by purpose
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Consent" }
components:
  responses:
    NotFound:
      description: Unknown principal
      content:
        application/problem+json:
          schema: { $ref: "#/components/schemas/Problem" }
  schemas:
    Problem:
      type: object
      required: [type, title, status, correlationId]
      properties:
        type: { type: string }
        title: { type: string }
        status: { type: integer }
        correlationId: { type: string }
    Principal:
      type: object
      required: [userId, role, status]
      properties:
        userId: { type: string, format: uuid }
        role: { type: string, enum: [user, admin, support] }
        status: { type: string, enum: [active, pending, deleted] }
    Consent:
      type: object
      required: [userId, analytics, marketing]
      properties:
        userId: { type: string, format: uuid }
        analytics: { type: boolean }
        marketing: { type: boolean }
```

## C4 — Asset Registry internal API (U1 → U4, U6, U7)

```yaml
openapi: 3.1.0
info:
  title: Asset Registry Internal API
  version: 1.0.0
paths:
  /v1/assets/resolve:
    get:
      operationId: resolveAsset
      parameters:
        - name: chain
          in: query
          required: true
          schema: { type: string }
        - name: contractAddress
          in: query
          required: false
          schema: { type: string }
        - name: symbol
          in: query
          required: false
          schema: { type: string }
      responses:
        "200":
          description: Canonical asset
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Asset" }
        "404": { $ref: "#/components/responses/Problem" }
components:
  responses:
    Problem:
      description: Unmapped observation
      content:
        application/problem+json:
          schema: { $ref: "#/components/schemas/Problem" }
  schemas:
    Problem:
      type: object
      required: [type, title, status, correlationId]
      properties:
        type: { type: string }
        title: { type: string }
        status: { type: integer }
        correlationId: { type: string }
    Asset:
      type: object
      required: [id, symbol, decimals, canonical]
      properties:
        id: { type: string }
        symbol: { type: string }
        decimals: { type: integer }
        canonical: { type: boolean }
```

## C5 — Account internal API (U4 → U6, U7, U11)

```yaml
openapi: 3.1.0
info:
  title: Account Internal API
  version: 1.0.0
paths:
  /v1/users/{userId}/holdings:
    get:
      operationId: getHoldings
      parameters:
        - name: userId
          in: path
          required: true
          schema: { type: string, format: uuid }
      responses:
        "200":
          description: Holdings per account
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/Holding" }
  /v1/accounts/{accountId}/transactions:
    get:
      operationId: getAccountTransactions
      parameters:
        - name: accountId
          in: path
          required: true
          schema: { type: string, format: uuid }
        - name: since
          in: query
          schema: { type: string, format: date-time }
      responses:
        "200":
          description: Venue transactions for the ledger
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/VenueTransaction" }
components:
  schemas:
    Holding:
      type: object
      required: [accountId, assetId, quantity, observedAt, priced]
      properties:
        accountId: { type: string, format: uuid }
        assetId: { type: string }
        quantity: { type: string }
        observedAt: { type: string, format: date-time }
        priced: { type: boolean }
    VenueTransaction:
      type: object
      required: [externalId, assetId, kind, quantity, executedAt]
      properties:
        externalId: { type: string }
        assetId: { type: string }
        kind: { type: string, enum: [buy, sell, transfer_in, transfer_out, fee] }
        quantity: { type: string }
        price: { type: string }
        executedAt: { type: string, format: date-time }
```

## C6 — Market Data internal API (U5 → U6, U8, U11)

```yaml
openapi: 3.1.0
info:
  title: Market Data Internal API
  version: 1.0.0
paths:
  /v1/prices:
    get:
      operationId: getPrices
      parameters:
        - name: assetIds
          in: query
          required: true
          schema:
            type: array
            items: { type: string }
      responses:
        "200":
          description: Prices with provenance and freshness
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/Price" }
  /v1/fx:
    get:
      operationId: getFxRate
      parameters:
        - name: base
          in: query
          required: true
          schema: { type: string }
        - name: quote
          in: query
          required: true
          schema: { type: string }
      responses:
        "200":
          description: FX rate
          content:
            application/json:
              schema: { $ref: "#/components/schemas/FxRate" }
components:
  schemas:
    Price:
      type: object
      required: [assetId, price, currency, source, observedAt, freshness]
      properties:
        assetId: { type: string }
        price: { type: string }
        currency: { type: string }
        source: { type: string }
        observedAt: { type: string, format: date-time }
        freshness: { type: string, enum: [live, delayed, stale, unpriced] }
    FxRate:
      type: object
      required: [base, quote, rate, observedAt]
      properties:
        base: { type: string }
        quote: { type: string }
        rate: { type: string }
        observedAt: { type: string, format: date-time }
```

## C7 — Ledger internal API (U7 → U6, U8)

```yaml
openapi: 3.1.0
info:
  title: Ledger Internal API
  version: 1.0.0
paths:
  /v1/users/{userId}/pnl:
    get:
      operationId: getPnl
      parameters:
        - name: userId
          in: path
          required: true
          schema: { type: string, format: uuid }
        - name: portfolioId
          in: query
          required: false
          schema: { type: string, format: uuid }
      responses:
        "200":
          description: P&L with coverage
          content:
            application/json:
              schema: { $ref: "#/components/schemas/Pnl" }
components:
  schemas:
    Pnl:
      type: object
      required: [asOf, netInvested, realized, unrealized, coverage]
      properties:
        asOf: { type: string, format: date-time }
        netInvested: { type: string }
        realized: { type: string }
        unrealized: { type: string }
        coverage: { type: number }
```

## C8 — Portfolio internal API (U6 → U8)

```yaml
openapi: 3.1.0
info:
  title: Portfolio Internal API
  version: 1.0.0
paths:
  /v1/portfolios/{portfolioId}/snapshots:
    get:
      operationId: getSnapshots
      parameters:
        - name: portfolioId
          in: path
          required: true
          schema: { type: string, format: uuid }
        - name: from
          in: query
          schema: { type: string, format: date-time }
      responses:
        "200":
          description: Immutable snapshots
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/Snapshot" }
components:
  schemas:
    Snapshot:
      type: object
      required: [id, asOf, totalValue, currency, inputsDigest]
      properties:
        id: { type: string, format: uuid }
        asOf: { type: string, format: date-time }
        totalValue: { type: string }
        currency: { type: string }
        inputsDigest: { type: string }
```

## C9 — Notification internal API (U9 → U4, U6, U8)

```yaml
openapi: 3.1.0
info:
  title: Notification Internal API
  version: 1.0.0
paths:
  /v1/alerts:
    post:
      operationId: submitAlert
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: "#/components/schemas/AlertRequest" }
      responses:
        "202":
          description: Accepted; deduplicated by idempotency key
          content:
            application/json:
              schema: { $ref: "#/components/schemas/AlertAccepted" }
components:
  schemas:
    AlertRequest:
      type: object
      required: [userId, type, idempotencyKey]
      properties:
        userId: { type: string, format: uuid }
        type: { type: string, enum: [drift, sync_failed, key_revoked, security, brief] }
        idempotencyKey: { type: string }
        payload: { type: object }
    AlertAccepted:
      type: object
      required: [alertId, status]
      properties:
        alertId: { type: string, format: uuid }
        status: { type: string, enum: [queued, deduplicated] }
```

## C10 — Audit internal API (U12 → every emitting unit)

```yaml
openapi: 3.1.0
info:
  title: Audit Internal API
  version: 1.0.0
paths:
  /v1/events:
    post:
      operationId: appendAuditEvent
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: "#/components/schemas/AuditEvent" }
      responses:
        "201":
          description: Appended
          content:
            application/json:
              schema: { $ref: "#/components/schemas/AuditEventAccepted" }
        "422": { $ref: "#/components/responses/Problem" }
components:
  responses:
    Problem:
      description: Rejected event
      content:
        application/problem+json:
          schema: { $ref: "#/components/schemas/Problem" }
  schemas:
    Problem:
      type: object
      required: [type, title, status, correlationId]
      properties:
        type: { type: string }
        title: { type: string }
        status: { type: integer }
        correlationId: { type: string }
    AuditEvent:
      type: object
      required: [action, target, outcome]
      properties:
        actorId: { type: string }
        action: { type: string }
        target: { type: string }
        occurredAt: { type: string, format: date-time }
        context: { type: object }
        outcome: { type: string, enum: [success, denied, failed] }
    AuditEventAccepted:
      type: object
      required: [id]
      properties:
        id: { type: string, format: uuid }
```

## C11 — Job run endpoint (each service → U11)

```yaml
openapi: 3.1.0
info:
  title: Service Run Endpoint
  version: 1.0.0
paths:
  /v1/internal/run/{jobName}:
    post:
      operationId: runJob
      description: Called by Job Orchestration with the shared service token.
      security:
        - serviceToken: []
      parameters:
        - name: jobName
          in: path
          required: true
          schema: { type: string }
      responses:
        "200":
          description: Run outcome
          content:
            application/json:
              schema: { $ref: "#/components/schemas/RunResult" }
        "409":
          description: Another run holds the single-flight lock
          content:
            application/problem+json:
              schema: { $ref: "#/components/schemas/Problem" }
components:
  securitySchemes:
    serviceToken:
      type: http
      scheme: bearer
  schemas:
    Problem:
      type: object
      required: [type, title, status, correlationId]
      properties:
        type: { type: string }
        title: { type: string }
        status: { type: integer }
        correlationId: { type: string }
    RunResult:
      type: object
      required: [jobName, status, startedAt]
      properties:
        jobName: { type: string }
        status: { type: string, enum: [succeeded, failed, skipped] }
        startedAt: { type: string, format: date-time }
        detail: { type: string }
```

## C12 — Analytics ingest API (U10 → U1)

```yaml
openapi: 3.1.0
info:
  title: Analytics Ingest API
  version: 1.0.0
paths:
  /v1/events:
    post:
      operationId: ingestEvent
      requestBody:
        required: true
        content:
          application/json:
            schema: { $ref: "#/components/schemas/AnalyticsEvent" }
      responses:
        "202":
          description: Accepted only when consent is granted
          content:
            application/json:
              schema: { $ref: "#/components/schemas/IngestAccepted" }
        "403":
          description: Consent not granted
          content:
            application/problem+json:
              schema: { $ref: "#/components/schemas/Problem" }
components:
  schemas:
    Problem:
      type: object
      required: [type, title, status, correlationId]
      properties:
        type: { type: string }
        title: { type: string }
        status: { type: integer }
        correlationId: { type: string }
    AnalyticsEvent:
      type: object
      required: [eventName, occurredAt]
      properties:
        eventName: { type: string }
        occurredAt: { type: string, format: date-time }
        properties: { type: object, description: no PII, credentials, or addresses }
    IngestAccepted:
      type: object
      required: [status]
      properties:
        status: { type: string, enum: [accepted] }
```

## C13–C14 — External contracts

Supabase Auth (OIDC/PKCE, JWKS) and WalletConnect Relay are external contracts owned by those
providers; the SPA consumes them and never modifies their shape. The gateway validates Supabase
JWTs against the JWKS endpoint; the SPA uses the Supabase SDK and the WalletConnect SDK.

## Contract ownership rules

- The **provider unit owns its spec**. A consumer may propose a change; the provider accepts or
  rejects it.
- **Versioning:** every contract is served under `/v1/`. A breaking change ships a new major
  version (`/v2/`) and both run until every consumer has migrated.
- **Additive changes** are safe: consumers ignore unknown fields, and the provider never removes
  or renames a field within a major version.
- **Errors** are always RFC 7807 `application/problem+json` with `type`, `title`, `status`,
  `detail`, and `correlationId`.
- **Timeouts and retries:** every internal call sets a connect timeout, a read timeout, and a
  total timeout; retries are bounded with exponential backoff and jitter, and carry an
  idempotency key where the call has a side effect.
- **Authentication:** user paths carry the Supabase JWT unchanged; job paths carry the shared
  service token from Vault.

## Open questions

| Contract | Question | Blocks |
|---|---|---|
| C1 | Exact pagination and filtering shape for the holdings and transactions lists | U1, U2 |
| C3 | Whether principal resolution caches in the gateway or is called per request | U1, U3 |
| C5 | Whether Account or Ledger owns CSV parsing and the import batch record | U4, U7 |
| C9 | Quiet-hours and digest windows in the alert submission contract | U9 |
| C11 | The exact job catalogue and per-job payload | U11 and every provider |
| C13 | Supabase Auth region and JWT audience/issuer values | U1, U2, U3 |
