import { describe, expect, it } from "vitest";

import {
  normalizeUsername,
  validateEmail,
  validatePassword,
  validateUsername,
} from "../validate-credentials";

describe("validateEmail", () => {
  it("rejects empty and malformed addresses, accepts a well-formed one", () => {
    expect(validateEmail("")).toBeTruthy();
    expect(validateEmail("   ")).toBeTruthy();
    expect(validateEmail("nope")).toBeTruthy();
    expect(validateEmail("a@b")).toBeTruthy(); // no TLD
    expect(validateEmail("a b@c.com")).toBeTruthy(); // space
    expect(validateEmail("you@company.com")).toBeNull();
  });
});

describe("validateUsername", () => {
  it("rejects capitals, spaces, too-short, and invalid characters", () => {
    expect(validateUsername("Chris")).toBeTruthy(); // capital
    expect(validateUsername("chris wolff")).toBeTruthy(); // space
    expect(validateUsername("ab")).toBeTruthy(); // < 3
    expect(validateUsername("chris!")).toBeTruthy(); // invalid char
    expect(validateUsername("a".repeat(31))).toBeTruthy(); // > 30
    expect(validateUsername("")).toBeTruthy();
    expect(validateUsername("chris_wolff-7")).toBeNull();
  });
});

describe("validatePassword", () => {
  it("enforces the 8-char floor for signup but only presence for login", () => {
    expect(validatePassword("")).toBeTruthy();
    expect(validatePassword("short")).toBeTruthy(); // < 8, signup
    expect(validatePassword("longenough")).toBeNull();

    // Login only requires a non-empty value — the server is the authority on correctness.
    expect(validatePassword("short", { forLogin: true })).toBeNull();
    expect(validatePassword("", { forLogin: true })).toBeTruthy();
  });
});

describe("normalizeUsername", () => {
  it("lowercases and strips whitespace so a capital or space can never persist", () => {
    expect(normalizeUsername("Chris Wolff")).toBe("chriswolff");
    expect(normalizeUsername("  ABC  ")).toBe("abc");
    expect(normalizeUsername("aB_c-1")).toBe("ab_c-1");
  });
});
