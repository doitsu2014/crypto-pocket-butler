# Security Requirements — u1-user-registry

**Upstream:** `functional-spec.md` (W1–W4), `rules.md` (BR1.5, BR1.8, BR1.9), `requirements.md`.

| ID | Requirement | Target | Source |
|---|---|---|---|
| NFR22.1 | Subject protection | The provider subject is stored only as a SHA-256 hash plus a deterministic pgsodium ciphertext; plaintext never persists | NFR22 |
| NFR9.1 | No secrets in logs or responses | No credential, MFA secret, or token appears in logs, responses, or error bodies | BRD-001 NFR9; AC1.5.2 |
| NFR10.1 | Token validation | Every authenticated call validates the Supabase JWT against JWKS | BRD-001 NFR10 |
| NFR23.1 | Least privilege | The API role reads and writes `users` and `shredding_keys` only; no cross-schema access | NFR23 |
| NFR23.2 | Admin boundary | An admin lookup returns the metadata boundary only, never portfolio contents or credentials | AC7.1.1; NFR23 |
| NFR9.2 | Input validation | Display name ≤ 200 chars; email in RFC 5322 shape; status restricted to the allowed set | BR1.4, BR1.10 |
| NFR19.1 | Audit | Provisioning, activation, and admin lookup are audited with actor (or system), action, target, timestamp, context, outcome; no credentials or tokens | BRD-001 NFR19 |

The subject is treated as personal data under BRD-017: the hash exists for lookup only, and the ciphertext exists so that erasure can destroy the ability to decrypt without deleting the lookup index.
