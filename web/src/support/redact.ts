/**
 * Local secret detection for the support form (NFR9.3a).
 *
 * The check is deliberately conservative and local: it refuses a message that
 * looks like it carries a credential, and never logs or transmits the matched
 * text. A normal sentence containing the word "key" passes.
 */

export interface SecretDetection {
  found: boolean;
  label?: string;
}

const RULES: Array<{ label: string; pattern: RegExp }> = [
  { label: "a twelve-word recovery phrase", pattern: /\b(?:[a-z]{2,10}\s+){11}[a-z]{2,10}\b/ },
  { label: "a twenty-four-word recovery phrase", pattern: /\b(?:[a-z]{2,10}\s+){23}[a-z]{2,10}\b/ },
  { label: "a private key", pattern: /\b0x[a-fA-F0-9]{64}\b/ },
  { label: "an extended private key", pattern: /\bxprv[a-zA-HJ-NP-Z1-9]{20,}\b/ },
  { label: "an extended public key", pattern: /\bxpub[a-zA-HJ-NP-Z1-9]{20,}\b/ },
  { label: "an API key", pattern: /\bsk-[A-Za-z0-9_-]{16,}\b/ },
  { label: "a publishable key", pattern: /\bpk_live_[A-Za-z0-9]{16,}\b/ },
  { label: "an AWS access key", pattern: /\bAKIA[A-Z0-9]{16}\b/ },
];

export function detectSecret(text: string): SecretDetection {
  for (const rule of RULES) {
    if (rule.pattern.test(text)) {
      return { found: true, label: rule.label };
    }
  }
  return { found: false };
}
