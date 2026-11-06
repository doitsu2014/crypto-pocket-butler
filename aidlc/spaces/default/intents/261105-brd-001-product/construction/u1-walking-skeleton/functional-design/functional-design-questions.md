# Functional Design — u1-walking-skeleton

This Unit is the walking skeleton plus the Asset Registry. These questions settle the behaviour
the design will specify. Answer each with a letter after `[Answer]:`, or pick `X` and say what
you want instead.

## Q1. [Architect] How should one asset be identified across chains?

- A. By chain plus contract address when one exists, falling back to the symbol only for native assets; one canonical asset can hold several contracts
- B. By symbol alone, preferring the highest market-cap match
- C. By contract address only; native assets need a sentinel contract
- X. Other (please specify)

[Answer]: A — Chain + contract

## Q2. [Architect] What happens when an observed token cannot be resolved?

- A. It goes to the quarantine as an unmapped observation, is excluded from portfolios, and is surfaced to the operator
- B. It is approximated to the closest symbol match
- C. It is dropped silently
- X. Other (please specify)

[Answer]: A — Quarantine + surface

## Q3. [Architect] How should price-source priority work?

- A. A configured priority list per asset; the first source that answers wins, with fallback to the next on failure
- B. Average every available source
- C. Lowest price wins
- X. Other (please specify)

[Answer]: A — Priority + fallback

## Q4. [Architect] What must the walking skeleton's gateway do before routing?

- A. Validate the Supabase JWT against JWKS, resolve the principal, enforce deny-by-default routing, add a correlation ID, and forward the JWT unchanged
- B. Forward any request and let each service validate
- C. Terminate authentication at the gateway only
- X. Other (please specify)

[Answer]: A — Full pre-routing
