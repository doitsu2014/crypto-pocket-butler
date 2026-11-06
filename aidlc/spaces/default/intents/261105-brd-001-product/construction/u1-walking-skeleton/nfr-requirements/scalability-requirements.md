# Scalability Requirements — u1-walking-skeleton

| ID | Requirement | Target | Parent |
|---|---|---|---|
| NFR20.1 | The gateway and registry scale horizontally as stateless replicas | add replicas with no sticky sessions; linear read scaling | NFR20 |
| NFR20.2 | Registry data volume | supports the launch asset set plus 10× growth without schema change | NFR20 |
| NFR20.3 | Quarantine growth is bounded | per-account cap with an operator review queue | NFR20 |
| NFR20.4 | Cache memory per replica is bounded | fixed maximum entry count with eviction | NFR20 |

## Notes

- The registry is read-heavy: a small data set read very often. Horizontal replicas plus the
  in-process cache cover launch volume.
- The launch capacity number is defined in the NFR Requirements of the platform work; this Unit
  states only that no design choice blocks horizontal scaling.
