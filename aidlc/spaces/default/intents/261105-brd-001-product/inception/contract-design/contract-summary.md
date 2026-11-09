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
| C6 | U5 Market Data | U6, U7, U8, U11 | HTTP/JSON, OpenAPI 3.1 | U5 |
| C7 | U7 Ledger | U6, U8 | HTTP/JSON, OpenAPI 3.1 | U7 |
| C8 | U6 Portfolio | U8 | HTTP/JSON, OpenAPI 3.1 | U6 |
| C9 | U9 Notification | U1, U4, U6, U8 | HTTP/JSON, OpenAPI 3.1 | U9 |
| C10 | U12 Audit | U1, U3, U4, U5, U6, U7, U8, U9, U10, U11 | HTTP/JSON, OpenAPI 3.1 | U12 |
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
      required: [portfolioId, asOf, status, currency, pricedCoverage, unpricedCount, lines]
      properties:
        portfolioId: { type: string, format: uuid }
        asOf: { type: string, format: date-time }
        status: { type: string, enum: [complete, partial, unavailable] }
        holdingsUnavailable: { type: boolean }
        totalValue: { type: [string, "null"], description: decimal string; null when unavailable }
        currency: { type: string }
        pricedCoverage: { type: number }
        unpricedCount: { type: integer }
        excludedCount: { type: integer, description: unpriced holdings kept as labelled lines }
        lines:
          type: array
          items: { $ref: "#/components/schemas/ValuationLine" }
    ValuationLine:
      type: object
      required: [assetId, quantity, dataStatus]
      properties:
        assetId: { type: string }
        quantity: { type: string }
        value: { type: [string, "null"] }
        allocation: { type: [number, "null"] }
        targetPercent: { type: [number, "null"] }
        driftPoints: { type: [number, "null"] }
        price: { type: [string, "null"] }
        priceSource: { type: [string, "null"] }
        priceFreshness: { type: string, enum: [live, delayed, stale, unpriced] }
        priceObservedAt: { type: [string, "null"], format: date-time }
        fxRate: { type: [string, "null"] }
        fxSource: { type: [string, "null"] }
        fxDerivedFrom: { type: array, items: { type: string } }
        fxObservedAt: { type: [string, "null"], format: date-time }
        dataStatus: { type: string, enum: [priced, unpriced, holdings_unavailable] }
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
        projectedAllocation: { type: number }
        projectedDriftPoints: { type: number }
        shortfall: { type: [string, "null"] }
        status: { type: string, enum: [pending, cancelled] }
        cancelledAt: { type: [string, "null"], format: date-time }
    TargetSetRequest:
      type: object
      required: [targets]
      properties:
        targets:
          type: array
          items:
            type: object
            required: [assetId, targetPercent]
            properties:
              assetId: { type: string }
              targetPercent: { type: number, minimum: 0, maximum: 100 }
              guardrailPoints: { type: number, exclusiveMinimum: 0 }
    TargetSetPreview:
      type: object
      required: [sum, difference]
      properties:
        sum: { type: number }
        difference: { type: number }
    TradeRequest:
      type: object
      required: [assetId, side, quantity, price, priceSource, executedAt, idempotencyKey]
      properties:
        assetId: { type: string }
        side: { type: string, enum: [buy, sell] }
        quantity: { type: string }
        price: { type: string }
        priceSource: { type: string, enum: [venue, user] }
        executedAt: { type: string, format: date-time }
        idempotencyKey: { type: string }
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

## C3 — Identity internal API (U3 → U1, U9, U10)

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

## C4 — Asset Registry internal API (U1 → U4, U5, U6, U7)

```yaml
openapi: 3.1.0
info:
  title: Asset Registry Internal API
  version: 1.0.0
security:
  - serviceToken: []
paths:
  /v1/assets:
    get:
      operationId: listAssets
      parameters:
        - name: tier
          in: query
          required: false
          schema: { type: string, enum: [top100, tail] }
        - name: limit
          in: query
          required: false
          schema: { type: integer, minimum: 1, maximum: 10000 }
      responses:
        "200":
          description: Canonical assets ordered by canonical rank
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/Asset" }
  /v1/price-sources:
    get:
      operationId: listPriceSources
      responses:
        "200":
          description: Enabled and disabled price sources with their priority
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/PriceSource" }
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
  securitySchemes:
    serviceToken:
      type: http
      scheme: bearer
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
      required: [id, symbol, decimals, canonical, rank]
      properties:
        id: { type: string }
        symbol: { type: string }
        decimals: { type: integer }
        canonical: { type: boolean }
        rank: { type: integer, minimum: 1 }
    PriceSource:
      type: object
      required: [name, kind, priority, quoteCurrency, rateLimitPerMinute, delayedFeed, enabled]
      properties:
        name: { type: string }
        kind: { type: string, enum: [exchange, aggregator, onchain] }
        priority: { type: integer, minimum: 1 }
        quoteCurrency: { type: string }
        rateLimitPerMinute: { type: integer, minimum: 1 }
        delayedFeed: { type: boolean }
        enabled: { type: boolean }
        licenceRef: { type: string }
```

Note: the canonical asset list and the price-source registry are owned by the Asset Registry
(U1); consumers read them through C4. `rank` orders the canonical set and defines the `top100`
tier; `PriceSource.priority` is the total order Market Data uses for selection.

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

## C6 — Market Data internal API (U5 → U6, U7, U8, U11)

```yaml
openapi: 3.1.0
info:
  title: Market Data Internal API
  version: 1.0.0
security:
  - serviceToken: []
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
            maxItems: 1000
            items: { type: string }
      responses:
        "200":
          description: Prices with provenance and freshness
          content:
            application/json:
              schema:
                type: array
                items: { $ref: "#/components/schemas/Price" }
        "401": { $ref: "#/components/responses/Problem" }
        "403": { $ref: "#/components/responses/Problem" }
        "422": { $ref: "#/components/responses/Problem" }
        "503": { $ref: "#/components/responses/Problem" }
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
        "401": { $ref: "#/components/responses/Problem" }
        "403": { $ref: "#/components/responses/Problem" }
        "404": { $ref: "#/components/responses/Problem" }
        "422": { $ref: "#/components/responses/Problem" }
        "503": { $ref: "#/components/responses/Problem" }
components:
  securitySchemes:
    serviceToken:
      type: http
      scheme: bearer
  responses:
    Problem:
      description: Validation, unavailable rate, or missing pair
      content:
        application/problem+json:
          schema:
            type: object
            required: [type, title, status, correlationId]
            properties:
              type: { type: string }
              title: { type: string }
              status: { type: integer }
              correlationId: { type: string }
  schemas:
    Price:
      type: object
      required: [assetId, freshness]
      properties:
        assetId: { type: string }
        price: { type: [string, "null"] }
        currency: { type: [string, "null"] }
        source: { type: [string, "null"] }
        observedAt: { type: [string, "null"], format: date-time }
        freshness: { type: string, enum: [live, delayed, stale, unpriced] }
      description: price, source, and observedAt are present for live/delayed/stale and null for unpriced.
    FxRate:
      type: object
      required: [base, quote, rate, observedAt, source]
      properties:
        base: { type: string }
        quote: { type: string }
        rate: { type: string }
        observedAt: { type: string, format: date-time }
        source: { type: string }
        derivedFrom:
          type: array
          items: { type: string }
          description: Both leg sources when the pair is derived through USD.
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
      required: [asOf, netInvested, realized, unrealized, coverage, currency, fxAsOf, fxEstimated, marksUnavailable, holdingsUnavailable, fxUnavailable]
      properties:
        asOf: { type: string, format: date-time }
        netInvested: { type: [string, "null"] }
        realized: { type: [string, "null"] }
        unrealized: { type: [string, "null"] }
        coverage: { type: number }
        currency: { type: string, description: The user's base currency; every figure is in it. }
        fxAsOf: { type: [string, "null"], format: date-time, description: The FX observation time used for the conversion. }
        fxEstimated: { type: boolean, description: True when any non-base amount was converted at the compute-time FX rate. }
        marksUnavailable: { type: boolean, description: True when C6 marks were unavailable; affected holdings are unpriced and the figures are withheld. }
        holdingsUnavailable: { type: boolean, description: True when C5 holdings were unavailable; coverage is named unavailable rather than guessed. }
        fxUnavailable: { type: boolean, description: True when a non-base amount could not be converted; no mixed-currency total is returned. }
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

## C9 — Notification internal API (U9 → U3, U4, U6, U8)

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
        "202":
          description: A run of this job is already active (manual path); runId names it
          content:
            application/json:
              schema: { $ref: "#/components/schemas/RunResult" }
        "409":
          description: Another run of this job holds the single-flight lock
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
        runId: { type: string }
        detail: { type: string }
```

`runId` is present when the provider created or found a run; the 409 response for a held
single-flight lock may carry `runId` in its problem extension so a manual caller can correlate.

## C12 — Analytics ingest API (U10 → U1)

```yaml
openapi: 3.1.0
info:
  title: Analytics Ingest API
  version: 1.0.0
security:
  - serviceToken: []
paths:
  /v1/events:
    post:
      operationId: ingestEvent
      description: The gateway validates the Supabase JWT and sets X-Auth-Subject; Analytics trusts that header only from the gateway and calls C3 getConsent with it.
      parameters:
        - name: X-Auth-Subject
          in: header
          required: true
          schema: { type: string, format: uuid }
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
        "401": { $ref: "#/components/responses/Problem" }
        "403":
          description: Consent not granted
          content:
            application/problem+json:
              schema: { $ref: "#/components/responses/Problem" }
        "422":
          description: Unknown event or disallowed property
          content:
            application/problem+json:
              schema: { $ref: "#/components/responses/Problem" }
  /v1/flags/{key}:
    get:
      operationId: getFlag
      description: Called by the gateway on behalf of an authenticated user; the gateway passes X-Auth-Subject.
      parameters:
        - name: key
          in: path
          required: true
          schema: { type: string }
      responses:
        "200":
          description: Flag evaluation
          content:
            application/json:
              schema:
                type: object
                required: [key, enabled]
                properties:
                  key: { type: string }
                  enabled: { type: boolean }
        "401": { $ref: "#/components/responses/Problem" }
  /v1/analytics/aggregates:
    get:
      operationId: getAggregates
      description: Admin only; the gateway checks the role.
      responses:
        "200": { description: Aggregates }
        "401": { $ref: "#/components/responses/Problem" }
        "403": { $ref: "#/components/responses/Problem" }
  /v1/admin/flags:
    post:
      operationId: createFlag
      responses:
        "201": { description: Created }
        "401": { $ref: "#/components/responses/Problem" }
        "403": { $ref: "#/components/responses/Problem" }
        "422": { $ref: "#/components/responses/Problem" }
  /v1/admin/flags/{key}:
    patch:
      operationId: updateFlag
      responses:
        "200": { description: Updated }
        "403": { $ref: "#/components/responses/Problem" }
    delete:
      operationId: deleteFlag
      responses:
        "204": { description: Deleted }
        "403": { $ref: "#/components/responses/Problem" }
  /v1/internal/smoke:
    get:
      operationId: smoke
      description: Release smoke; validates the service token and dependency reachability.
      responses:
        "200": { description: Ready }
        "401": { $ref: "#/components/responses/Problem" }
  /v1/internal/privacy/erase:
    post:
      operationId: eraseUserEvents
      description: Identity's erasure orchestration deletes this user's pseudonymous events.
      requestBody:
        required: true
        content:
          application/json:
            schema:
              type: object
              required: [authUserId]
              properties:
                authUserId: { type: string, format: uuid }
      responses:
        "204": { description: Erased }
        "401": { $ref: "#/components/responses/Problem" }
        "403": { $ref: "#/components/responses/Problem" }
components:
  securitySchemes:
    serviceToken:
      type: http
      scheme: bearer
  responses:
    Problem:
      description: Validation, consent, or unavailable
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

## Amendment — Identity (U3) contract change requests

Recorded 2026-11-07 from the u3-identity functional design review. These additions are
provider-owned: the owning unit accepts or rejects each, and a consumer may propose but not
impose it.

### C1 — SPA-facing Identity routes

The gateway exposes these to the signed-in SPA (carrying the Supabase JWT), plus one
pre-authentication route:

| Route | Purpose |
|---|---|
| `POST /v1/consent` | Append a consent decision |
| `PATCH /v1/me/preferences` | Locale, base currency, timezone |
| `GET /v1/me/sessions` | Device session list |
| `DELETE /v1/me/sessions/{sessionId}` | Revoke a session |
| `POST /v1/exports` | Start an export |
| `GET /v1/exports/{exportId}` | Progress and signed download |
| `POST /v1/privacy/deletions` | Confirm account deletion |
| `GET /v1/privacy/deletions/{deletionId}` | Deletion state |
| `POST /v1/support-grants` | Request a support grant |
| `POST /v1/support-grants/{grantId}/approve` | Admin approval |
| `DELETE /v1/support-grants/{grantId}` | Admin revoke |
| `POST /v1/signup-gate` | Pre-authentication jurisdiction and age gate; the only C1 route that does not require a JWT |

### C3 — Identity internal API additions

Every C3 path carries the forwarded Supabase JWT; Identity validates it against Supabase JWKS
itself. The principal response gains the next required step so the gateway can route a pending
user.

```yaml
Principal:
  type: object
  required: [userId, role, status, nextStep]
  properties:
    userId: { type: string, format: uuid }
    role: { type: string, enum: [user, admin, support] }
    status: { type: string, enum: [pending, active, deleted] }
    nextStep: { type: string, enum: [none, verify_email, enrol_mfa] }
```

`POST /v1/signup-gate` lives here for the pre-authentication call and returns
`{ "gateToken": "<signed, short-lived>" }` or an RFC 7807 refusal.

### C2 — Support access additions

- `getAccountMetadata` gains a required `grantId` query parameter.
- `SupportAccessGrant` status values become `pending_approval`, `active`, `expired`, `revoked`,
  and the schema adds `requestedAt`, `approvedBy`, `approvedAt`; `expiresAt` is set on approval.
- The approve and revoke operations are the Identity C1 routes above.

### C5, C7, C8 — Privacy operations

| Contract | Operation | Purpose |
|---|---|---|
| C5 Account | `GET /v1/internal/privacy/export` (service token, `userId`) | Accounts and holdings for export |
| C5 Account | `POST /v1/internal/privacy/anonymise` (service token, `userId`, idempotency key) | Anonymise, keep labelled retention records |
| C7 Ledger | `GET /v1/internal/privacy/export` | Transactions for export |
| C7 Ledger | `POST /v1/internal/privacy/anonymise` | Anonymise, label retained financial records |
| C8 Portfolio | `GET /v1/internal/privacy/export` | Portfolios and snapshots for export |
| C8 Portfolio | `POST /v1/internal/privacy/anonymise` | Anonymise personal portfolio fields |

### C7 — Ledger additions and market valuation

- `GET /v1/users/{userId}/transactions` (extension) returns the append-only rows with `currency`
  and `priceMissing`; a missing price is `null`, never zero.
- `GET /v1/users/{userId}/pnl` computes every figure in the user's base currency. It returns
  `currency`, `fxAsOf` (the FX observation time), and `fxEstimated` (true when a non-base amount
  was converted at the compute-time rate).
- `assets[]` carries `assetId`, `covered`, `markPrice` (nullable), `priceAsOf`, and
  `priceFreshness` (`live`, `delayed`, `stale`, `unpriced`). An `unpriced` holding is named in
  `coverageGap` and excluded from the figures; it is never valued at zero.
- Coverage is computed from Account's C5 holdings and valued with C6 prices: every held asset
  without acquisition history is named in the gap, `coverage` is covered value / total held value,
  and P&L is unavailable when no holding has history.
- A C5 or C6 outage is a degraded **200**, never a 503: affected holdings are labelled
  `unpriced`, `coverageLabel` is `partial` with the gap naming them, the figures are withheld, and
  `marksUnavailable`, `holdingsUnavailable`, or `fxUnavailable` is set. Only a Postgres failure
  returns 503.
- The C6 read-path call is one attempt bounded to 150 ms, assuming a C6 p95 of at most 120 ms, so
  the 400 ms `getPnl` budget holds. A read that returns a labelled degraded answer (complete,
  partial, or unpriced) counts as **good** in the P&L availability SLI; only a 5xx counts as bad.
- Ledger is a C6 consumer (`GET /v1/prices`, `GET /v1/fx`), so C6's consumer list gains U7.

### C10 — Audit event required fields

`actorId`, `occurredAt`, and `context` become required on `AuditEvent`, matching the NFR19
requirement that every event carries actor, action, target, timestamp, context, and outcome.

### C11 — Job Orchestration calls Identity

Identity exposes `POST /v1/internal/run/audit-privacy-sweep` (service token) so Job Orchestration
can resume export and deletion work after a restart, and records Identity as a provider.

### C15 — Supabase Auth hooks and admin (external)

- A pre-sign-up hook verifies the signed gate token from `POST /v1/signup-gate`; a sign-up without
  a valid token is refused.
- The admin user-delete endpoint removes the Auth identity, password, and MFA factors on deletion.

### C16 — Supabase Storage export bucket (external)

Identity writes the export archive to a private `identity-exports` bucket: encrypted at rest,
owner-only signed download, object deleted after the 24-hour download window. Identity is a
consumer of Supabase Storage.

### Account and Identity claim

Account maintains `app_metadata.has_exchange_credentials` on the Supabase Auth user when a
credential is stored or removed, using its Auth admin authority; Identity reads the claim from the
validated JWT. A stale claim is corrected on the next credential verification or removal.

## Amendment — Account (U4) contract change requests

Recorded 2026-11-07 from the u4-account functional design review.

### C1 — SPA-facing Account routes (U1-owned)

| Route | Purpose |
|---|---|
| `POST /v1/accounts/exchange` | Add an exchange account with a read-only key |
| `POST /v1/accounts/wallet` | Add a public wallet address |
| `POST /v1/accounts/walletconnect` | Attach a WalletConnect read-only connection |
| `POST /v1/accounts/{accountId}/resync` | Manual re-sync (202 with the active run id) |
| `GET /v1/users/{userId}/accounts` | Accounts with status and last-synced time |
| `POST /v1/accounts/{accountId}/imports` | CSV upload |

C1's `Account` schema gains `kind: walletconnect`, `venue` values `binance`, `okx`, `evm:<chainId>`,
`solana`, `csv:<label>`, and `status` values `active`, `syncing`, `error`, `disconnected`.

### C4 — Asset Registry supported-chains read

`GET /v1/chains` returns the supported EVM chains and Solana, each with its chain id, native symbol,
and RPC endpoint reference. Account reads it to validate a wallet chain; a newly registered chain is
selectable on the next read with no Account redeploy.

### C5 — Account additions

- `Holding` makes `assetId` optional, adds `observedToken`, `spam: boolean`, and `lastSyncedAt:
  date-time`; `priced` leaves Account's response (Portfolio's read carries it), and Account does not
  return `dust`.
- `getAccountTransactions` returns venue and imported transactions; imported rows carry
  `externalId` and the ledger-relevant fields.
- Ledger discovers a user's accounts through `getHoldings` and then reads each account's
  transactions. An account with no holdings and no transactions is not visited; a user-level
  accounts read would close that gap if it is added later.
- `GET /v1/internal/privacy/export` returns the account rows, labels, wallet addresses, holdings,
  and imported transactions for the user; `POST /v1/internal/privacy/anonymise` removes the labels
  and wallet addresses, anonymises the personal fields, keeps the labelled financial-retention rows,
  and returns the retained categories.
- CSV ownership is settled here: **Account owns `ImportBatch`, `ImportedTransaction`, and
  `ImportRowError`** and provides the import surface; u7-ledger delivers US1.4's acceptance criteria
  and consumes the imported rows through C5.

### C11 — Scheduled sync semantics

`POST /v1/internal/run/account-sync` returns 409 while an account's run lease is live; a manual
re-sync through C1 returns 202 with the active run id. A run whose lease expires is marked failed and
releases the guard.

### C15 — Account uses the Auth admin claim update

Account calls the Supabase Auth admin API to set `app_metadata.has_exchange_credentials` on store and
to clear it on rejection, removal, or revocation with no remaining verified credential.

### C1 — Reporting privacy (U3 → U8)

`GET /v1/internal/privacy/export?userId=` returns the user's briefs and deliveries; `POST
/v1/internal/privacy/anonymise` with userId and an idempotency key replaces the userId with an
opaque reference while briefs remain financial records. Identity's service token authorizes both,
and Reporting is added to Identity's deletion contributor set.

### C3 — Reporting user selection (U8 → U3)

`GET /v1/internal/users/due?period=daily|weekly&at=<rfc3339>` with Reporting's service token
returns `[{userId, timezone}]` for users whose local period boundary has passed; Reporting then
applies its own skip/retry rule. Identity is the source of user enumeration and timezone, and its
registry must allow-list Reporting for this operation. U9 must honour the new `channel` field; a
mandatory security/key_revoked email ignores an in-app-only channel. U6 must add the C8 reads and
allow-list Reporting for the portfolios and valuation operations.

### C9 — Alert channel (any emitter → U9)

`AlertRequest` gains `channel` (`in_app|email|both`, default `both`). A brief uses `email` so it does
not add a second in-app entry; Notification suppresses email per preference but keeps in-app for
`both`.

### C1 — SPA-facing Reporting routes (U1 → U2)

- `GET /v1/users/{userId}/briefs?period=&cursor=` lists the caller's briefs (owner-only).
- `GET /v1/users/{userId}/briefs/{briefId}` opens one brief with its figures, as-of, and data
  status (owner-only).
Brief schema: id, period (daily|weekly), periodStart/periodEnd, status (pending|generated|failed),
failureReason, asOf, dataStatus (complete|partial|unavailable), sections (valuation lines with
provenance, drift, P&L with coverage), generatedAt.

### C8 — Reporting-facing valuation reads (U6 → U8)

- `GET /v1/users/{userId}/portfolios` lists the user's portfolios (service-token Reporting).
- `GET /v1/portfolios/{portfolioId}/valuation` returns `PortfolioValuation` including the per-asset
  `lines` with price source/freshness/time, FX provenance, targets, and drift.
- `GET /v1/portfolios/{portfolioId}/snapshots` items include their stored `lines`.

### C1 — SPA-facing Portfolio routes (U1 → U2)

- `POST /v1/portfolios` creates a portfolio; `GET /v1/portfolios` and `GET /v1/portfolios/{id}`
  return the list and one portfolio with valuation, breakdown, and drift.
- `PUT /v1/portfolios/{id}/accounts/{accountId}` assigns or moves an account.
- `PUT /v1/portfolios/{id}/targets` saves targets and guardrails; with `?preview=true` it returns
  the sum and difference from 100 without saving.
- `POST /v1/portfolios/{id}/order-intents` creates an intent; with `?preview=true` it returns the
  projected allocation, drift, and any shortfall without recording.
- `POST /v1/portfolios/{id}/order-intents/{intentId}/cancel` cancels a pending intent.
- `POST /v1/portfolios/{id}/trades` records an executed trade.
- `GET /v1/portfolios/{id}/snapshots` returns snapshot history (distinct from the C8 internal route).

### C7 — Ledger records a confirmed trade (U6 → U7)

`POST /v1/users/{userId}/trades` records a user-confirmed executed trade on Ledger: asset, side,
quantity, price, `priceSource` (venue|user), executedAt, and a client idempotency key. It returns
201 with the ledger transaction id; a replay with the same key returns the existing entry.

### C1 — SPA-facing Notification routes (U1 → U2)

- `GET /v1/users/{userId}/notifications?cursor=&limit=` returns the caller's in-app alerts
  (owner-only; the gateway forwards the JWT and enforces the owner).
- `GET /v1/users/{userId}/notifications/preferences` and `PUT .../preferences` read and save
  per-type, per-channel preferences (owner-only; a mandatory security/key_revoked email cannot be
  disabled).
Both are rate-limited per user at the gateway.

### C9 — Notification erasure (U3 → U9)

Identity's erasure orchestration calls `POST /v1/internal/privacy/anonymise` on Notification with its
service token, the userId, and an idempotency key; Notification replaces the userId on that user's
alerts and delivery attempts with a non-reversible reference, keeps the twelve-month operational
rows, and returns 204. A repeat with the same key is idempotent.

### C3 — Notification contact read (U9)

`GET /v1/users/{userId}/contact` returns the user's verified email address and a `verified` flag
from Identity; Notification calls it at delivery time with its service token. A missing or
unverified address is an `address` delivery error and no email is sent. FR7.3 billing-type email is
out of this release (no Billing unit); one-click unsubscribe applies to the brief email only and is
exempt for mandatory security and credential email.

### C2 — Support console routes (U1 → U13)

- `GET /v1/support/accounts?grantId=&query=` searches account metadata by id or label within the
  grant's target user only (support role; grantId required).
- `GET /v1/support/accounts/{accountId}/metadata?grantId=` reads account metadata. The free-text
  read reason travels in the `X-Support-Reason` request header, never in the URL; the support API
  hashes it for the C10 `support_metadata_read` event and never returns or logs it.
- `GET /v1/support/tickets?grantId=&status=` lists ticket metadata within the grant's target user
  only (support role; grantId required). It returns a pseudonymous `requesterRef`, never a
  requester label or any ticket conversation content.
- `POST /v1/support-grants` creates a pending grant (support role).
- `POST /v1/support-grants/{grantId}/approve` (second approver; must differ from the requester) and
  `DELETE /v1/support-grants/{grantId}` (admin revoke) act on a grant.
- `GET /v1/support-grants/{grantId}` returns status and `expiresAt`.
- `POST /v1/support/incidents`, `PATCH /v1/support/incidents/{id}` declare/update an incident
  (`admin` only).
- `GET /v1/support/incidents?active=true` is read by u2-web-app's public status page.

The account metadata read validates that the account's owner is the grant's target user; a grant for
one user cannot read another user's account. The incident store, its C10 audit and the C9
notification are owned by U1's support API, not by the console; the console is UI-only. Incident has
`severity` (sev1/sev2/sev3) and `status` (declared/investigating/resolved) as separate fields.

#### C2 amendment — reason header and grant-scoped tickets

Recorded 2026-11-08 from the u13 code-generation review.

- The metadata read's required `reason` moves from the query string to the `X-Support-Reason`
  request header. A URL-carried reason lands in gateway and nginx access logs, browser history,
  and referrers; the header keeps it out of all three. The gateway forwards the header unchanged
  and must not log it. The support API hashes the reason for C10 (BR2.1/BR2.2) and never echoes it.
- `GET /v1/support/tickets` requires `grantId` and returns only ticket metadata for accounts owned
  by that grant's target user. `TicketMetadata.requesterLabel` is removed; a pseudonymous
  `requesterRef` may be returned, and conversation content is never part of the DTO.
- `SupportAccessGrant` carries the additive `requestedBy` field so the console can disable
  self-approval; the support API remains the enforcement point.

### C1 — SPA-facing Analytics routes (U1 → U2)

- `GET /v1/flags/{key}` evaluates one flag for the caller's JWT subject and returns
  `{key, enabled}`; it is open to any authenticated user.
- `GET /v1/analytics/aggregates?metric=&period=&from=&to=` returns aggregates; it requires the
  `admin` role from `app_metadata.role` and a gateway admin guard.
- `POST /v1/admin/flags`, `PATCH /v1/admin/flags/{key}`, and `DELETE /v1/admin/flags/{key}`
  administer flags; they require the `admin` role.

### C3 — Analytics consent and erasure

Analytics calls `GET /v1/consent/{userId}` with its service token before every non-essential ingest;
a missing or withdrawn analytics grant returns 403. On Identity's erasure orchestration, Identity
calls Analytics `POST /v1/internal/privacy/erase` with the pseudonymous `userRef`; Analytics deletes
the events and returns 204.

### C3 — Identity support-grant check

`GET /v1/support-grants/{grantId}/check` returns `active`, `expired`, or `revoked` for the support
user and target user. Account calls it on a support metadata read and refuses when it is not active.
Identity is added to Account's dependencies for this check.

## Amendment — Job Orchestration (U11) run call and catalogue

Recorded 2026-11-08 from the u11-job-orchestration functional design review.

### C11 — Run call shape

Every provider's `POST /v1/internal/run/{jobName}` accepts an optional body `{"owner": "<instance>"}`
and the header `idempotency-key: job:{job}:{scheduledFor}` (canonical whole-second UTC RFC 3339).
The provider stores the outcome per key: a duplicate whose recorded outcome is `succeeded` or
`skipped` returns that outcome without re-running; a duplicate whose recorded outcome is `failed`
re-executes the window and replaces the outcome. The key is per window (not per attempt), so a
retried call reaches the same row and a succeeded side effect is applied once (AC9.1.3).

| Answer | Meaning |
|---|---|
| 200 with `{status: succeeded\|failed\|skipped, ...}` | the provider ran (or replayed a completed key); the status is recorded as-is |
| 202 with `{runId}` | the provider already has a live run for the window; the orchestrator records `skipped` with the id |
| 409 | a per-item lease is held (Account's per-account lease uses this); retried within the bound |
| other 4xx | a misconfigured call; abandoned, not retried |

This resolves the open C11 question for the run call's payload and dedupe key. The per-job payload
stays the job name plus `owner`; no provider needs a richer body.

### C11 — Job catalogue (U11 → providers)

| Job | Provider |
|---|---|
| `account-sync` | Account |
| `market-data-refresh`, `market-data-fx` | Market Data |
| `portfolio-valuation`, `portfolio-snapshot` | Portfolio |
| `ledger-reconcile`, `ledger-ingest` | Ledger |
| `analytics-aggregate`, `analytics-flag-expiry` | Analytics |
| `notification-delivery`, `notification-digest` | Notification |
| `brief-generation` | Reporting |
| `audit-privacy-sweep` | Identity |
| `audit-retention` | Audit |

Each provider's service-token registry must allow-list `job-orchestration` for its run endpoint and
accept the job name. The orchestrator validates the forwarded Supabase JWT against JWKS itself and
enforces the `admin`/`support` role on its operator paths; the gateway adds rate limits.


## Amendment — Web App (U2) C1 routes and PKCE storage

Recorded 2026-11-08 from the u2-web-app NFR design review.

- `GET /v1/users/me` returns `mfaRequired` and `mfaEnrolled`; `POST /v1/mfa/enrol` and
  `POST /v1/mfa/recovery` (U3) enrol MFA and return recovery codes once.
- `POST /v1/telemetry/client-errors` is a public, rate-limited C1 route (no JWT) accepting
  `{route, errorCode, correlationId}` with no payload; it is the SPA's only pre-sign-in ingress.
- The SPA's PKCE code verifier may live in `sessionStorage` for the duration of a callback; access
  and refresh tokens never leave memory. This is the single stated exception to BR3.1.
