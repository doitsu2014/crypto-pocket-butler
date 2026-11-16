# Personas — Authentication & Session Management

Four personas. Each story references one of them; a story that fits none is either the wrong story
or a missing persona.

---

## P1 — Maya, the account holder

**Role:** A private individual using Crypto Pocket Butler to keep track of a crypto portfolio. Not
a security specialist, and does not want to become one.

**Goals**
- Get into the product quickly from whatever device she is on.
- Stay signed in without re-entering anything during a working session.
- Feel that her holdings are not exposed because she left a browser open somewhere.

**Pain points**
- Sign-in friction: a password manager, a second device, a forgotten password.
- Distrust of anything that looks like it is storing credentials in the browser.
- No idea whether her phone from last year is still signed in.

**Context:** signs in on a laptop daily and on a phone several times a week. Expects sessions to
survive a page reload but to end when she signs out. Has never read a security notice in her life
and does not plan to start.

**Tech comfort:** medium. Uses password managers, understands "sign out of all devices".

**Frequency:** daily.

**Priority rank:** 1.

---

## P2 — Maya, locked out or suspicious

*The same person, at the moment something has gone wrong. Kept separate because the needs are
opposite to P1's: P1 wants a fast, frictionless path; P2 wants a slow, deliberate, reversible one,
and is willing to pay friction for certainty.*

**Goals**
- Get back in after losing access to the email account or the device.
- Understand what happened, in plain words, when an alert about a new device arrives.
- Take control back immediately — end every session but this one.

**Pain points**
- Being asked to prove identity by a channel she has lost.
- An error message that says "something went wrong" when what she needs to know is whether her
  account is safe.
- Not knowing whether a recovery she started actually worked.

**Context:** contacts support only after trying everything else. May be in a hurry and may be
frightened.

**Tech comfort:** drops to low under stress.

**Frequency:** rare, but the highest-stakes moments in the product.

**Priority rank:** 2.

---

## P3 — Sam, the support agent

**Role:** Front-line support, works to a supervisor, handles identity and access cases.

**Goals**
- Help a locked-out user recover without ever seeing or handling a secret.
- Do it in a way that is fully recorded, so a later review can reconstruct exactly what happened.
- Not be the person who can unilaterally take over an account.

**Pain points**
- Recovery that depends on the user's own second channel, which is gone by definition.
- Ambiguity about what he may and may not reveal to a caller.
- Being asked to "just reset it" without an audit trail.

**Context:** has a queue, a supervisor, and an audit service that will be read later. Works from a
support console, not the SPA. Every action he takes is visible to someone else.

**Tech comfort:** medium-high in the console, no access to the database.

**Frequency:** daily.

**Priority rank:** 3.

---

## P4 — Dana, the administrator

**Role:** Owns user administration and incident response. Acts on other people's accounts.

**Goals**
- End a compromised user's sessions quickly, with a record of why.
- Disable an account without destroying its history.
- Rely on the system to have already refused a disabled user's token.

**Pain points**
- Actions that succeed silently and leave no trail.
- Having to infer whether a revocation actually took effect.
- Needing to act on an account but being unable to confirm which sessions are live.

**Context:** uses the support console and the admin routes. Works to policy, not to a script.
Answers to the organisation when something goes wrong.

**Tech comfort:** high.

**Frequency:** several times a week.

**Priority rank:** 4.

---

## Persona relationships

```
Maya (P1)  --signs in, manages own devices-->  product
Maya (P1)  --becomes-->  Maya in trouble (P2)  --contacts-->  Sam (P3)
Sam (P3)   --acts under supervision, audited-->  product
Dana (P4)  --acts on any account, by policy-->  product
```

**Text fallback:** Maya signs in and manages her own devices. When she is locked out or worried, she
contacts Sam, who performs an audited recovery under supervision. Dana can act on any account by
policy, including ending sessions.

## Notes on the set

- **P2 is not a duplicate of P1.** The requirements differ: P1 gets a fast sign-in and a stable
  session; P2 gets a slow, audited, support-mediated path. Collapsing them would have produced
  stories where a security story also had to be convenient.
- **P3 and P4 are separated by authority.** Sam cannot approve his own recovery; Dana can act
  unilaterally but must leave a trail. The stories that touch another user's session differ in who
  may do it, so they carry different personas.
- **No compliance persona.** The jurisdiction list and the retention policy are owned outside the
  product surface and appear in the requirements as constraints, not as a user of the sign-in flow.
