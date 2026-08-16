import { ChatBubble } from "@/components/chat";

const transcript = [
  { role: "user" as const, content: "Summarize this 50-page document in 3 bullets." },
  {
    role: "assistant" as const,
    content: "Done — Meridian completed the request with the model enabled for this project.",
  },
];

const trace = ["✓ model supported", "↳ project spend checked", "● prepaid credit verified"];

/** Landing feature-highlight for the in-app assistant. Presentational - sells the capability. */
export function Assistant() {
  return (
    <section aria-labelledby="assistant-heading" className="border-border border-t">
      <div className="mx-auto grid w-full max-w-6xl items-center gap-12 px-6 py-20 lg:grid-cols-2">
        <div className="flex flex-col gap-5">
          <span className="text-muted-foreground font-mono text-xs tracking-[0.2em] uppercase">
            Intelligent Routing
          </span>
          <h2
            id="assistant-heading"
            className="font-display text-3xl font-semibold tracking-tight md:text-4xl"
          >
            Route smart. Pay less.
          </h2>
          <p className="text-muted-foreground max-w-md text-lg leading-relaxed">
            GaussMeridian automatically selects the best model for each request. Save on costs,
            optimize for speed, and track every decision with full observability.
          </p>
          <ul className="text-muted-foreground flex flex-col gap-2 font-mono text-sm">
            <li>✓ automatic provider selection</li>
            <li>✓ semantic and exact-match caching</li>
            <li>● usage tracking and cost attribution</li>
          </ul>
        </div>

        <div className="bg-card border-border shadow-glow rounded-2xl border p-5">
          <div className="flex flex-col gap-3">
            {transcript.map((message) => (
              <ChatBubble key={message.content} message={message} />
            ))}
          </div>
          <div className="border-border text-muted-foreground mt-4 flex flex-wrap gap-x-4 gap-y-1 border-t pt-3 font-mono text-xs">
            {trace.map((line) => (
              <span key={line}>{line}</span>
            ))}
          </div>
        </div>
      </div>
    </section>
  );
}
