import { describe, expect, it } from "vitest";

import { CAPITAL_POOL, latLonToVec3, pickRoute } from "../route";

describe("CAPITAL_POOL", () => {
  it("has around 22 capitals, each with a name and finite coordinates", () => {
    expect(CAPITAL_POOL.length).toBeGreaterThanOrEqual(20);
    for (const capital of CAPITAL_POOL) {
      expect(capital.name.length).toBeGreaterThan(0);
      expect(Number.isFinite(capital.lat)).toBe(true);
      expect(Number.isFinite(capital.lon)).toBe(true);
    }
  });
});

describe("pickRoute", () => {
  it("returns n distinct cities drawn from the pool", () => {
    const route = pickRoute(CAPITAL_POOL, 7);
    expect(route).toHaveLength(7);

    const names = new Set(route.map((c) => c.name));
    expect(names.size).toBe(7);
    for (const capital of route) {
      expect(CAPITAL_POOL).toContainEqual(capital);
    }
  });

  it("defaults to n = 7 when no count is given", () => {
    expect(pickRoute(CAPITAL_POOL)).toHaveLength(7);
  });

  it("clamps n to the pool size rather than repeating cities", () => {
    const route = pickRoute(CAPITAL_POOL, 1000);
    expect(route).toHaveLength(CAPITAL_POOL.length);
    expect(new Set(route.map((c) => c.name)).size).toBe(CAPITAL_POOL.length);
  });

  it("does not mutate the pool passed in", () => {
    const before = [...CAPITAL_POOL];
    pickRoute(CAPITAL_POOL, 7);
    expect(CAPITAL_POOL).toEqual(before);
  });

  it("produces a different order/selection across calls (probabilistically)", () => {
    const samples = Array.from({ length: 10 }, () =>
      pickRoute(CAPITAL_POOL, 7)
        .map((c) => c.name)
        .join(","),
    );
    const distinct = new Set(samples);
    // Flaky only if 10 independent 7-of-22 draws collide every time — astronomically unlikely.
    expect(distinct.size).toBeGreaterThan(1);
  });
});

describe("latLonToVec3", () => {
  it("is unit-length at r = 1 for arbitrary lat/lon", () => {
    for (const capital of CAPITAL_POOL) {
      const { x, y, z } = latLonToVec3(capital.lat, capital.lon, 1);
      const length = Math.sqrt(x * x + y * y + z * z);
      expect(length).toBeCloseTo(1, 10);
    }
  });

  it("maps the north pole (lat 90) to the +y axis", () => {
    const { x, y, z } = latLonToVec3(90, 0, 1);
    expect(x).toBeCloseTo(0, 10);
    expect(y).toBeCloseTo(1, 10);
    expect(z).toBeCloseTo(0, 10);
  });

  it("maps the south pole (lat -90) to the -y axis", () => {
    const { x, y, z } = latLonToVec3(-90, 0, 1);
    expect(x).toBeCloseTo(0, 10);
    expect(y).toBeCloseTo(-1, 10);
    expect(z).toBeCloseTo(0, 10);
  });

  it("maps the equator at lon 0 to +x (the blue-marble texture seam convention)", () => {
    const { x, y, z } = latLonToVec3(0, 0, 1);
    expect(x).toBeCloseTo(1, 10);
    expect(y).toBeCloseTo(0, 10);
    expect(z).toBeCloseTo(0, 10);
  });

  it("scales linearly with radius", () => {
    const unit = latLonToVec3(12.3, -45.6, 1);
    const scaled = latLonToVec3(12.3, -45.6, 5);
    expect(scaled.x).toBeCloseTo(unit.x * 5, 10);
    expect(scaled.y).toBeCloseTo(unit.y * 5, 10);
    expect(scaled.z).toBeCloseTo(unit.z * 5, 10);
  });
});
