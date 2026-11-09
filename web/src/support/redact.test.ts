import { describe, expect, it } from "vitest";
import { detectSecret } from "./redact";

const secretCases: Array<[string, string]> = [
  [
    "a twelve-word mnemonic",
    "abandon ability able about above absent absorb abstract absurd abuse access accident",
  ],
  [
    "a twenty-four-word mnemonic",
    "abandon ability able about above absent absorb abstract absurd abuse access accident account accuse achieve acid acoustic acquire across act action actor actress actual",
  ],
  ["an EVM private key", `0x${"a".repeat(64)}`],
  ["an extended private key", `xprv${"a".repeat(80)}`],
  ["an extended public key", `xpub${"a".repeat(80)}`],
  ["an OpenAI-style key", `sk-${"a".repeat(32)}`],
  ["a live publishable key", `pk_live_${"a".repeat(24)}`],
  ["an AWS access key", `AKIA${"A".repeat(16)}`],
];

describe("secret detection", () => {
  it.each(secretCases)("blocks %s", (_label, text) => {
    expect(detectSecret(text).found).toBe(true);
  });

  it("names the field label for a detected secret", () => {
    expect(detectSecret(`0x${"b".repeat(64)}`).label).toMatch(/private key/i);
  });

  it("passes a normal sentence containing the word key", () => {
    expect(detectSecret("My API key stopped working after I rotated it.").found).toBe(
      false,
    );
  });

  it("passes a short list of words that is not a mnemonic", () => {
    expect(detectSecret("one two three four five six").found).toBe(false);
  });
});
