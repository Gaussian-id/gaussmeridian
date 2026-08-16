import { ScrollReveal } from "@/components/motion";

/** Representative OpenAI-compatible response body; supplier implementation stays private. */
const lines: { k?: string; v: string; muted?: boolean; note?: string }[] = [
  { v: "{", muted: true },
  { k: '  "id":', v: '"chatcmpl_8f1c…",' },
  { k: '  "object":', v: '"chat.completion",' },
  { k: '  "model":', v: '"google/gemini-2.5-flash",' },
  { k: '  "choices":', v: "[ … ]," },
  { k: '  "usage":', v: '{ "prompt_tokens": 812, "completion_tokens": 146, "total_tokens": 958 }' },
  { v: "}", muted: true, note: "  // one normalized contract" },
];

export function HeadersSection() {
  return (
    <section className="mx-auto flex min-h-dvh max-w-6xl flex-col justify-center px-6 py-20">
      <ScrollReveal>
        <h2 className="font-display max-w-[20ch] text-3xl font-semibold tracking-tight md:text-4xl">
          The request stays <span className="text-gradient">traceable</span>.
        </h2>
        <p className="text-muted-foreground mt-3 max-w-[52ch] text-base">
          Read one normalized response shape, then correlate the request with project activity and
          the organization that funded access. Meridian keeps its supplier connection behind the
          contract.
        </p>
      </ScrollReveal>
      <ScrollReveal delay={0.05}>
        <div className="bg-brand-gradient shadow-glow mt-7 max-w-3xl overflow-x-auto rounded-2xl p-5 font-mono text-[13px] leading-loose text-white">
          {lines.map((l, i) => (
            <div key={i}>
              {l.k && <span className="text-[var(--gauss-400)]">{l.k} </span>}
              <span className={l.muted ? "text-white/50" : "text-white"}>{l.v}</span>
              {l.note && <span className="text-white/50">{l.note}</span>}
            </div>
          ))}
        </div>
      </ScrollReveal>
    </section>
  );
}
