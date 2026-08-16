/**
 * The imperative three.js scene behind `<MeridianField>` (PRD-22 Phase B) — a realistic,
 * reactive Earth: custom day/night `ShaderMaterial` (blue-marble day, night-lights + city glow,
 * ocean specular), a headlight sun that tracks the camera, a faint meridian graticule, a
 * starfield, and a satellite-style camera orbit choreographed by the onboarding step, with
 * capital nodes / great-circle arcs / projected name labels revealing as the flight progresses.
 *
 * Framework-free by design (no React) so it can be dynamically imported — this file, and the
 * `three` module it pulls in, only load once `<MeridianField>` mounts client-side, keeping three
 * out of the initial bundle. Re-implements the approved prototype's `makeEarth()` 1:1 (same
 * constants, same shader math); see `scene-helpers.ts` for the pure step/reveal/sun logic this
 * class calls into each frame.
 *
 * Lifecycle mirrors `components/motion/meridian-globe.tsx`: one `WebGLRenderer`, one RAF loop,
 * `dispose()` tears down every geometry/material/texture/renderer and detaches the label DOM
 * nodes it created — callers must call `dispose()` on unmount.
 */

import * as THREE from "three";

import type { OnboardingStep } from "@/lib/onboarding/onboarding-machine";
import type { Capital } from "@/lib/onboarding/route";
import { latLonToVec3 } from "@/lib/onboarding/route";

import {
  cameraTargetForStep,
  headlightSunDirection,
  isArcRevealed,
  isLabelVisible,
  isNodeRevealed,
  offsetXForWidth,
} from "./scene-helpers";

const EARTH_RADIUS = 2;
const NODE_COLORS = [0x5ad1ff, 0x7b5cff, 0xc084fc] as const; // gauss v2 / v1 / v3, prototype order
const STAR_COUNT = 1500;
const ARC_SEGMENTS = 54;
const CAMERA_LERP_K = 0.045;
const NODE_REVEAL_LERP_K = 0.1;
const ARC_OPACITY_LERP_K = 0.06;
const GRATICULE_SPIN_PER_FRAME = 0.0002;
const STAR_SPIN_PER_SECOND = 0.002;
const ARC_OPACITY_TARGET = 0.85;
const TEXTURE_BASE = "/textures/earth";

const EARTH_VERTEX_SHADER = `
  varying vec2 vUv;
  varying vec3 vN;
  varying vec3 vW;
  void main() {
    vUv = uv;
    vN = normalize(mat3(modelMatrix) * normal);
    vec4 wp = modelMatrix * vec4(position, 1.0);
    vW = wp.xyz;
    gl_Position = projectionMatrix * viewMatrix * wp;
  }
`;

const EARTH_FRAGMENT_SHADER = `
  precision highp float;
  uniform sampler2D dayMap, nightMap, specMap;
  uniform vec3 uSun;
  uniform float uHasDay, uHasNight, uHasSpec;
  varying vec2 vUv;
  varying vec3 vN;
  varying vec3 vW;
  void main() {
    vec3 N = normalize(vN);
    float lam = dot(N, normalize(uSun));
    float day = smoothstep(-0.4, 0.28, lam);
    vec3 draw = uHasDay > 0.5 ? texture2D(dayMap, vUv).rgb : vec3(.16, .3, .52);
    vec3 lit = draw * (1.45 + 0.3 * clamp(lam, 0.0, 1.0));
    vec3 nl = uHasNight > 0.5 ? texture2D(nightMap, vUv).rgb * 1.2 : vec3(0.0);
    vec3 night = draw * 0.85 + nl;
    float sp = 0.0;
    if (uHasSpec > 0.5) {
      vec3 V = normalize(cameraPosition - vW);
      vec3 H = normalize(normalize(uSun) + V);
      sp = pow(max(dot(N, H), 0.0), 18.0) * texture2D(specMap, vUv).r * day;
    }
    vec3 col = mix(night, lit, day) + vec3(.8, .85, .7) * sp * .55;
    gl_FragColor = vec4(col, 1.0);
  }
`;

interface CityMark {
  city: Capital;
  dir: THREE.Vector3;
  pos: THREE.Vector3;
  mesh: THREE.Mesh;
  halo: THREE.Mesh;
  label: HTMLDivElement;
  reveal: number; // eased 0..1
}

export interface MeridianFieldSceneOptions {
  canvas: HTMLCanvasElement;
  /** Positioned (`position: relative|absolute`) container the scene appends label `<div>`s into. */
  labelsContainer: HTMLElement;
  route: Capital[];
  initialStep: OnboardingStep;
  reducedMotion: boolean;
}

export class MeridianFieldScene {
  private readonly renderer: THREE.WebGLRenderer;
  private readonly scene = new THREE.Scene();
  private readonly camera: THREE.PerspectiveCamera;
  private readonly group = new THREE.Group();
  private readonly center = new THREE.Vector3();
  private readonly earthUniforms: {
    dayMap: { value: THREE.Texture | null };
    nightMap: { value: THREE.Texture | null };
    specMap: { value: THREE.Texture | null };
    uSun: { value: THREE.Vector3 };
    uHasDay: { value: number };
    uHasNight: { value: number };
    uHasSpec: { value: number };
  };
  private readonly earthGeometry: THREE.SphereGeometry;
  private readonly earthMaterial: THREE.ShaderMaterial;
  private readonly graticuleGeometry: THREE.WireframeGeometry;
  private readonly graticuleMaterial: THREE.LineBasicMaterial;
  private readonly graticule: THREE.LineSegments;
  private readonly starGeometry: THREE.BufferGeometry;
  private readonly starMaterial: THREE.PointsMaterial;
  private readonly stars: THREE.Points;
  private readonly textures: THREE.Texture[] = [];
  private readonly marks: CityMark[];
  private readonly arcs: THREE.Line[];
  private readonly avgDir: THREE.Vector3;
  private readonly route: Capital[];
  private readonly labelsContainer: HTMLElement;

  private step: OnboardingStep;
  private reducedMotion: boolean;
  private offsetX = -1.9;
  private curDir: THREE.Vector3;
  private curDist = 5;
  private raf = 0;
  private disposed = false;
  private readonly clock = new THREE.Clock();
  private readonly tmpProjected = new THREE.Vector3();

  constructor(opts: MeridianFieldSceneOptions) {
    this.route = opts.route;
    this.step = opts.initialStep;
    this.reducedMotion = opts.reducedMotion;
    this.labelsContainer = opts.labelsContainer;

    this.renderer = new THREE.WebGLRenderer({ canvas: opts.canvas, antialias: true });
    this.renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this.renderer.outputColorSpace = THREE.SRGBColorSpace;

    this.camera = new THREE.PerspectiveCamera(45, 1, 0.1, 300);
    this.group.position.copy(this.center);
    this.scene.add(this.group);

    // Starfield.
    const starPositions = new Float32Array(STAR_COUNT * 3);
    for (let i = 0; i < STAR_COUNT; i++) {
      const r = 40 + Math.random() * 90;
      const a = Math.random() * Math.PI * 2;
      const b = Math.acos(2 * Math.random() - 1);
      starPositions[i * 3] = r * Math.sin(b) * Math.cos(a);
      starPositions[i * 3 + 1] = r * Math.sin(b) * Math.sin(a);
      starPositions[i * 3 + 2] = r * Math.cos(b);
    }
    this.starGeometry = new THREE.BufferGeometry().setAttribute(
      "position",
      new THREE.BufferAttribute(starPositions, 3),
    );
    this.starMaterial = new THREE.PointsMaterial({
      color: 0x9fb0ff,
      size: 0.18,
      transparent: true,
      opacity: 0.75,
      depthWrite: false,
    });
    this.stars = new THREE.Points(this.starGeometry, this.starMaterial);
    this.scene.add(this.stars);

    // Earth: day/night shader, headlight sun.
    const initialSunDir = new THREE.Vector3(-4, 2, 4).normalize();
    this.earthUniforms = {
      dayMap: { value: null },
      nightMap: { value: null },
      specMap: { value: null },
      uSun: { value: initialSunDir },
      uHasDay: { value: 0 },
      uHasNight: { value: 0 },
      uHasSpec: { value: 0 },
    };
    const loader = new THREE.TextureLoader();
    loader.load(`${TEXTURE_BASE}/blue-marble.jpg`, (t) => {
      t.colorSpace = THREE.SRGBColorSpace;
      this.earthUniforms.dayMap.value = t;
      this.earthUniforms.uHasDay.value = 1;
      this.textures.push(t);
    });
    loader.load(`${TEXTURE_BASE}/night.jpg`, (t) => {
      t.colorSpace = THREE.SRGBColorSpace;
      this.earthUniforms.nightMap.value = t;
      this.earthUniforms.uHasNight.value = 1;
      this.textures.push(t);
    });
    loader.load(`${TEXTURE_BASE}/water.png`, (t) => {
      this.earthUniforms.specMap.value = t;
      this.earthUniforms.uHasSpec.value = 1;
      this.textures.push(t);
    });
    this.earthMaterial = new THREE.ShaderMaterial({
      uniforms: this.earthUniforms,
      vertexShader: EARTH_VERTEX_SHADER,
      fragmentShader: EARTH_FRAGMENT_SHADER,
    });
    this.earthGeometry = new THREE.SphereGeometry(EARTH_RADIUS, 72, 72);
    this.group.add(new THREE.Mesh(this.earthGeometry, this.earthMaterial));

    // Meridian graticule — faint wireframe, no atmosphere glow.
    this.graticuleGeometry = new THREE.WireframeGeometry(
      new THREE.SphereGeometry(EARTH_RADIUS * 1.003, 24, 16),
    );
    this.graticuleMaterial = new THREE.LineBasicMaterial({
      color: 0x5ad1ff,
      transparent: true,
      opacity: 0.07,
    });
    this.graticule = new THREE.LineSegments(this.graticuleGeometry, this.graticuleMaterial);
    this.group.add(this.graticule);

    // Capital nodes, halos, and DOM labels.
    this.marks = this.route.map((city, i) => {
      const dirVec = latLonToVec3(city.lat, city.lon, 1);
      const dir = new THREE.Vector3(dirVec.x, dirVec.y, dirVec.z);
      const pos = dir.clone().multiplyScalar(EARTH_RADIUS * 1.008);
      const color = NODE_COLORS[i % NODE_COLORS.length];

      const mesh = new THREE.Mesh(
        new THREE.SphereGeometry(0.032, 16, 16),
        new THREE.MeshBasicMaterial({ color }),
      );
      const halo = new THREE.Mesh(
        new THREE.SphereGeometry(0.085, 16, 16),
        new THREE.MeshBasicMaterial({
          color,
          transparent: true,
          opacity: 0,
          blending: THREE.AdditiveBlending,
        }),
      );
      mesh.add(halo);
      mesh.position.copy(pos);
      mesh.scale.setScalar(0.001);
      this.group.add(mesh);

      const label = document.createElement("div");
      label.textContent = city.name;
      Object.assign(label.style, {
        position: "absolute",
        transform: "translate(-50%, -150%)",
        fontSize: "11px",
        fontWeight: "600",
        letterSpacing: "0.03em",
        color: "#e6ecff",
        whiteSpace: "nowrap",
        padding: "2px 8px",
        borderRadius: "7px",
        background: "rgba(8,10,20,.6)",
        border: "1px solid rgba(255,255,255,.14)",
        opacity: "0",
        transition: "opacity .35s",
        pointerEvents: "none",
      });
      this.labelsContainer.appendChild(label);

      return { city, dir, pos, mesh, halo, label, reveal: 0 };
    });

    // Great-circle arcs between consecutive cities.
    this.arcs = [];
    for (let i = 1; i < this.route.length; i++) {
      const from = this.marks[i - 1].dir;
      const to = this.marks[i].dir;
      const points: THREE.Vector3[] = [];
      for (let s = 0; s <= ARC_SEGMENTS; s++) {
        const t = s / ARC_SEGMENTS;
        const p = from.clone().lerp(to, t).normalize();
        p.multiplyScalar(EARTH_RADIUS * (1 + 0.2 * Math.sin(Math.PI * t)));
        points.push(p);
      }
      const line = new THREE.Line(
        new THREE.BufferGeometry().setFromPoints(points),
        new THREE.LineBasicMaterial({
          color: 0x8fd6ff,
          transparent: true,
          opacity: 0,
          blending: THREE.AdditiveBlending,
        }),
      );
      this.group.add(line);
      this.arcs.push(line);
    }

    this.avgDir =
      this.marks.length > 0
        ? this.marks.reduce((sum, m) => sum.add(m.dir), new THREE.Vector3()).normalize()
        : new THREE.Vector3(0, 0, 1);

    const startDir = this.marks[0]?.dir.clone() ?? new THREE.Vector3(0, 0, 1);
    this.curDir = startDir;
    this.camera.position.copy(this.center).add(this.curDir.clone().multiplyScalar(this.curDist));
    this.camera.lookAt(this.center);

    this.resize(window.innerWidth, window.innerHeight);
    this.raf = requestAnimationFrame(this.frame);
  }

  setStep(step: OnboardingStep): void {
    this.step = step;
  }

  setReducedMotion(reducedMotion: boolean): void {
    this.reducedMotion = reducedMotion;
  }

  resize(width: number, height: number): void {
    this.renderer.setSize(width, height, false);
    this.camera.aspect = width / Math.max(height, 1);
    this.camera.updateProjectionMatrix();
    this.offsetX = offsetXForWidth(width);
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    cancelAnimationFrame(this.raf);

    this.earthGeometry.dispose();
    this.earthMaterial.dispose();
    this.graticuleGeometry.dispose();
    this.graticuleMaterial.dispose();
    this.starGeometry.dispose();
    this.starMaterial.dispose();
    this.textures.forEach((t) => t.dispose());

    for (const mark of this.marks) {
      mark.mesh.geometry.dispose();
      (mark.mesh.material as THREE.Material).dispose();
      mark.halo.geometry.dispose();
      (mark.halo.material as THREE.Material).dispose();
      mark.label.remove();
    }

    for (const arc of this.arcs) {
      arc.geometry.dispose();
      (arc.material as THREE.Material).dispose();
    }

    this.renderer.dispose();
  }

  private readonly frame = (): void => {
    if (this.disposed) return;
    const t = this.clock.getElapsedTime();
    const routeLength = this.route.length;

    this.group.position.set(this.offsetX, 0, 0);
    this.center.set(this.offsetX, 0, 0);

    if (routeLength > 0) {
      const target = cameraTargetForStep(this.step, routeLength);
      const targetDir = target.pullBack ? this.avgDir : this.marks[target.cityIndex].dir;
      const targetDist = target.distance;

      const k = this.reducedMotion ? 1 : CAMERA_LERP_K;
      this.curDir.lerp(targetDir, k).normalize();
      this.curDist += (targetDist - this.curDist) * k;
    }

    this.camera.position.copy(this.center).add(this.curDir.clone().multiplyScalar(this.curDist));
    this.camera.lookAt(this.center);

    if (!this.reducedMotion) {
      this.graticule.rotation.y += GRATICULE_SPIN_PER_FRAME;
      this.stars.rotation.y = t * STAR_SPIN_PER_SECOND;
    }

    const camDir = this.camera.position.clone().sub(this.center).normalize();
    const sun = headlightSunDirection({ x: camDir.x, y: camDir.y, z: camDir.z });
    this.earthUniforms.uSun.value.set(sun.x, sun.y, sun.z);

    for (let i = 0; i < this.marks.length; i++) {
      const mark = this.marks[i];
      const revealed = isNodeRevealed(this.step, routeLength, i);
      const revealK = this.reducedMotion ? 1 : NODE_REVEAL_LERP_K;
      mark.reveal += ((revealed ? 1 : 0) - mark.reveal) * revealK;
      mark.mesh.scale.setScalar(0.001 + mark.reveal);
      (mark.halo.material as THREE.MeshBasicMaterial).opacity =
        0.55 * mark.reveal * (this.reducedMotion ? 0.6 : 0.6 + 0.4 * Math.sin(t * 3 + i));

      const frontDot = mark.dir.dot(camDir);
      const visible = isLabelVisible({ revealed, frontDot, revealAmount: mark.reveal });
      if (visible) {
        this.tmpProjected.copy(mark.pos);
        this.tmpProjected.x += this.offsetX;
        this.tmpProjected.project(this.camera);
        mark.label.style.left = `${(this.tmpProjected.x * 0.5 + 0.5) * window.innerWidth}px`;
        mark.label.style.top = `${(-this.tmpProjected.y * 0.5 + 0.5) * window.innerHeight}px`;
        mark.label.style.opacity = "1";
      } else {
        mark.label.style.opacity = "0";
      }
    }

    for (let i = 0; i < this.arcs.length; i++) {
      const revealed = isArcRevealed(this.step, routeLength, i + 1);
      const target = revealed ? ARC_OPACITY_TARGET : 0;
      const opacityK = this.reducedMotion ? 1 : ARC_OPACITY_LERP_K;
      const material = this.arcs[i].material as THREE.LineBasicMaterial;
      material.opacity += (target - material.opacity) * opacityK;
    }

    this.renderer.render(this.scene, this.camera);
    this.raf = requestAnimationFrame(this.frame);
  };
}
