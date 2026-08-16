import type { MoaCandidate } from "@core/adapters/schemas/console.schema";

/**
 * Global (project-independent) GaussMoA candidate panel fixture for the Playground (M5).
 * Exactly one winner; every loser is stamped `stamped_cost: 0`.
 */
export const moaCandidates: MoaCandidate[] = [
  {
    model: "gpt-4o",
    provider: "openai",
    contribution: 0.55,
    is_winner: true,
    stamped_cost: 0.0421,
  },
  {
    model: "claude-3-5-sonnet",
    provider: "anthropic",
    contribution: 0.3,
    is_winner: false,
    stamped_cost: 0,
  },
  {
    model: "gemini-1.5-pro",
    provider: "google",
    contribution: 0.15,
    is_winner: false,
    stamped_cost: 0,
  },
];
