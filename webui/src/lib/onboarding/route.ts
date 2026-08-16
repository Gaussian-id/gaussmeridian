/**
 * Onboarding "Meridian Earth" route — a pure, unit-testable module (PRD-22 §4). Supplies the
 * per-session capital route the wizard flies the satellite camera through: a fixed pool of world
 * capitals, an unbiased random subset picker, and the lat/lon → sphere projection the 3D scene
 * (Phase B) will consume. No React, no three.js — just data + math, so randomness and geometry
 * are each independently verifiable.
 */

export interface Capital {
  name: string;
  lat: number;
  lon: number;
}

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

/** ~22 world capitals spanning every inhabited continent, for a visually varied route. */
export const CAPITAL_POOL: readonly Capital[] = [
  { name: "Washington", lat: 38.9072, lon: -77.0369 },
  { name: "Ottawa", lat: 45.4215, lon: -75.6972 },
  { name: "Mexico City", lat: 19.4326, lon: -99.1332 },
  { name: "Brasília", lat: -15.7939, lon: -47.8828 },
  { name: "Buenos Aires", lat: -34.6037, lon: -58.3816 },
  { name: "London", lat: 51.5074, lon: -0.1278 },
  { name: "Paris", lat: 48.8566, lon: 2.3522 },
  { name: "Madrid", lat: 40.4168, lon: -3.7038 },
  { name: "Berlin", lat: 52.52, lon: 13.405 },
  { name: "Rome", lat: 41.9028, lon: 12.4964 },
  { name: "Cairo", lat: 30.0444, lon: 31.2357 },
  { name: "Abuja", lat: 9.0765, lon: 7.3986 },
  { name: "Pretoria", lat: -25.7479, lon: 28.2293 },
  { name: "Moscow", lat: 55.7558, lon: 37.6173 },
  { name: "Riyadh", lat: 24.7136, lon: 46.6753 },
  { name: "New Delhi", lat: 28.6139, lon: 77.209 },
  { name: "Beijing", lat: 39.9042, lon: 116.4074 },
  { name: "Seoul", lat: 37.5665, lon: 126.978 },
  { name: "Tokyo", lat: 35.6762, lon: 139.6503 },
  { name: "Singapore", lat: 1.3521, lon: 103.8198 },
  { name: "Jakarta", lat: -6.2088, lon: 106.8456 },
  { name: "Canberra", lat: -35.2809, lon: 149.13 },
] as const;

/**
 * An unbiased random subset of `n` distinct capitals from `pool`, via Fisher–Yates (partial
 * shuffle — only the first `n` slots are settled). `n` is clamped to `pool.length`. Does not
 * mutate `pool`.
 */
export function pickRoute(pool: readonly Capital[], n = 7): Capital[] {
  const count = Math.min(n, pool.length);
  const shuffled = [...pool];

  for (let i = 0; i < count; i++) {
    const j = i + Math.floor(Math.random() * (shuffled.length - i));
    [shuffled[i], shuffled[j]] = [shuffled[j], shuffled[i]];
  }

  return shuffled.slice(0, count);
}

/**
 * Projects a lat/lon onto a sphere of radius `r`, in the standard blue-marble texture
 * orientation (equirectangular UV origin at the antimeridian): `phi = (90 - lat)°`,
 * `theta = (lon + 180)°`, `x = -sin(phi)cos(theta)`, `y = cos(phi)`, `z = sin(phi)sin(theta)`.
 */
export function latLonToVec3(lat: number, lon: number, r: number): Vec3 {
  const phi = ((90 - lat) * Math.PI) / 180;
  const theta = ((lon + 180) * Math.PI) / 180;

  return {
    x: -r * Math.sin(phi) * Math.cos(theta),
    y: r * Math.cos(phi),
    z: r * Math.sin(phi) * Math.sin(theta),
  };
}
