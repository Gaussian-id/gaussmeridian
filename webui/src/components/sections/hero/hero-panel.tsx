/** The signature device: a live "audit stream" — Gaussian's product made visible. */
const auditStream = [
  { mark: "✓", label: "action proven" },
  { mark: "✓", label: "boundary held" },
  { mark: "●", label: "data in-perimeter" },
  { mark: "↳", label: "supervisor approved" },
];

/** Horizontal audit strip — centered under the hero CTAs, wraps gracefully on mobile. */
export function HeroPanel() {
  return (
    <div className="bg-brand-gradient shadow-glow mt-2 flex w-full max-w-2xl flex-wrap items-center justify-center gap-x-6 gap-y-2 rounded-2xl px-6 py-4 text-white">
      <span className="font-mono text-[10px] tracking-[0.25em] text-white/55 uppercase">
        live audit stream
      </span>
      {auditStream.map((row) => (
        <span key={row.label} className="flex items-center gap-2 font-mono text-sm">
          <span className="text-[var(--gauss-400)]">{row.mark}</span>
          <span className="text-white/85">{row.label}</span>
        </span>
      ))}
    </div>
  );
}
