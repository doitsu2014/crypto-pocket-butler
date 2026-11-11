/**
 * The `/gates` route module (functional-spec §2.2; performance-design §3).
 *
 * Lazily loaded so the country module and the form stay in the gate route
 * chunk, never the entry chunk.
 *
 * Requirements: FR19.3, NFR2.1, NFR2.4.
 */

import { SignupGatesScreen, type GateClient } from "./SignupGatesScreen";

export default function SignupGatesPage({ client }: { client?: GateClient } = {}) {
  return <SignupGatesScreen client={client} />;
}
