import "@testing-library/jest-dom/vitest";
import { vi } from "vitest";

// next-themes reads matchMedia; jsdom doesn't implement it.
Object.defineProperty(window, "matchMedia", {
  writable: true,
  value: (query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    addEventListener: vi.fn(),
    removeEventListener: vi.fn(),
    dispatchEvent: vi.fn(),
  }),
});

// jsdom implements neither observer — needed by any surface that mounts scroll/viewport-
// aware motion (Reveal, parallax/tilt hooks) or virtualized/lazy content (M2+ console surfaces).
class NoopObserver {
  observe = vi.fn();
  unobserve = vi.fn();
  disconnect = vi.fn();
  takeRecords = vi.fn(() => []);
}
vi.stubGlobal("IntersectionObserver", NoopObserver);
vi.stubGlobal("ResizeObserver", NoopObserver);

// jsdom has no canvas backend; components that probe for a WebGL context (MeridianGlobe,
// BrandOrb) must see `null`, the same as a real browser without WebGL support, not throw.
Object.defineProperty(HTMLCanvasElement.prototype, "getContext", {
  writable: true,
  value: () => null,
});

// jsdom doesn't implement smooth-scroll navigation; components that call it (e.g. the
// command palette scrolling a result into view) would otherwise throw "not implemented".
Object.defineProperty(window, "scrollTo", {
  writable: true,
  value: vi.fn(),
});

// jsdom doesn't implement Element.scrollIntoView either — `cmdk` (the M5 command palette's
// list) calls it on the selected item as the user arrows/types, which would otherwise throw
// "not a function" the instant the palette mounts.
Object.defineProperty(Element.prototype, "scrollIntoView", {
  writable: true,
  value: vi.fn(),
});

// jsdom has no `EventSource` at all — `useRouteDecisionStream` (the PRD-21 Wave C live feed)
// opens one unconditionally wherever `RecentRoutesFeed` mounts, which would otherwise throw
// "EventSource is not defined" the instant the Overview page renders in any test. This is a
// deliberately inert stub (never fires `onopen`/`onmessage`) — a test that needs to exercise a
// specific connection state constructs its own richer fake and calls `vi.stubGlobal` locally.
class NoopEventSource {
  static readonly CONNECTING = 0;
  static readonly OPEN = 1;
  static readonly CLOSED = 2;
  onopen: (() => void) | null = null;
  onmessage: ((event: MessageEvent) => void) | null = null;
  onerror: (() => void) | null = null;
  close = vi.fn();
  addEventListener = vi.fn();
  removeEventListener = vi.fn();
}
vi.stubGlobal("EventSource", NoopEventSource);
