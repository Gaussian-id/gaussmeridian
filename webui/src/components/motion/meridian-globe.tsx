"use client";

import { useEffect, useRef } from "react";
import * as THREE from "three";

import { themeConfig } from "@theme/theme.config";

/**
 * The signature background device: a single wireframe "meridian" globe, rendered once for the
 * whole marketing surface. It drifts on its own, turns with scroll, and steers toward the
 * cursor — so the meridian lines stretch across the screen as you move and read.
 *
 * One WebGL context, disposed on unmount. Under `prefers-reduced-motion` (or coarse pointers)
 * the globe renders static — the brand mark is present, but nothing animates.
 *
 * Placed once in the marketing layout as a fixed, non-interactive background (`z-0`); page
 * content sits above it in a `relative z-10` wrapper, so transparent sections reveal the globe.
 */
export function MeridianGlobe() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    const fine = window.matchMedia("(pointer: fine)").matches;
    const interactive = !reduce && fine;

    const renderer = new THREE.WebGLRenderer({ canvas, alpha: true, antialias: true });
    renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(48, 1, 0.1, 100);
    camera.position.z = 3.05;

    const geometry = new THREE.SphereGeometry(2.15, 30, 20);
    const material = new THREE.MeshBasicMaterial({
      color: new THREE.Color(themeConfig.palette.gauss500),
      wireframe: true,
      transparent: true,
      opacity: 0.2,
    });
    const globe = new THREE.Mesh(geometry, material);
    globe.position.x = 0.35;
    scene.add(globe);

    const state = { px: 0, py: 0, x: 0, y: 0, scroll: 0, auto: 0 };

    const resize = () => {
      const w = window.innerWidth;
      const h = window.innerHeight;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      camera.updateProjectionMatrix();
      const small = w < 620;
      material.opacity = small ? 0.14 : 0.2;
      globe.scale.setScalar(small ? 0.82 : 1);
      globe.position.x = small ? 0 : 0.35;
    };
    resize();

    const onPointer = (e: PointerEvent) => {
      state.px = (e.clientX / window.innerWidth - 0.5) * 2;
      state.py = (e.clientY / window.innerHeight - 0.5) * 2;
    };
    const onScroll = () => {
      state.scroll = window.scrollY || document.documentElement.scrollTop;
    };

    if (interactive) window.addEventListener("pointermove", onPointer, { passive: true });
    window.addEventListener("scroll", onScroll, { passive: true });
    window.addEventListener("resize", resize);

    let frame = 0;
    const render = (t: number) => {
      state.x += (state.px - state.x) * 0.06;
      state.y += (state.py - state.y) * 0.06;
      if (!reduce) state.auto += 0.0008;
      globe.rotation.y = state.auto + state.scroll * 0.0012 + (interactive ? state.x * 0.5 : 0);
      globe.rotation.x = 0.16 + Math.sin(t * 0.00018) * 0.045 + (interactive ? state.y * 0.28 : 0);
      renderer.render(scene, camera);
      frame = requestAnimationFrame(render);
    };
    frame = requestAnimationFrame(render);

    return () => {
      cancelAnimationFrame(frame);
      if (interactive) window.removeEventListener("pointermove", onPointer);
      window.removeEventListener("scroll", onScroll);
      window.removeEventListener("resize", resize);
      geometry.dispose();
      material.dispose();
      renderer.dispose();
    };
  }, []);

  return (
    <canvas
      ref={canvasRef}
      aria-hidden="true"
      className="pointer-events-none fixed inset-0 z-0 h-full w-full"
    />
  );
}
