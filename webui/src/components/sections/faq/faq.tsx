import { ChevronDown } from "lucide-react";

const faqs = [
  {
    q: "What does GaussMeridian do?",
    a: "GaussMeridian is an intelligent LLM API gateway. It acts as a proxy between your application and LLM providers, enabling smart routing, caching, cost optimization, and full observability.",
  },
  {
    q: "How does prepaid pricing work?",
    a: "Add a fixed amount of credit to an organization whenever you need it. Verified purchased credit unlocks model requests, and nothing renews or recharges automatically.",
  },
  {
    q: "What is GaussMoA (mixture-of-agents)?",
    a: "GaussMeridian's intelligent routing layer evaluates each request and automatically selects the best-fit model across all your configured providers - optimizing for cost, latency, or quality based on your preferences.",
  },
  {
    q: "Where does my data live?",
    a: "GaussMeridian keeps project identity, routing evidence, and usage records inside the product contract. Payment credentials remain with the regulated payment processor and are never stored by Meridian.",
  },
  {
    q: "How do I get started?",
    a: "Create an account, choose an organization and project, add prepaid credit, then create a project API key or use the Playground. Unfunded requests stop before model work begins.",
  },
  {
    q: "What about caching?",
    a: "GaussMeridian includes semantic caching (cosine similarity >= 0.95) and exact-match caching powered by SurrealDB. Cache hits are free and reduce both latency and costs.",
  },
];

export function Faq() {
  return (
    <section aria-labelledby="faq-heading" className="border-border border-t">
      <div className="mx-auto w-full max-w-3xl px-6 py-20">
        <div className="text-center">
          <span className="text-muted-foreground font-mono text-xs tracking-[0.2em] uppercase">
            FAQ
          </span>
          <h2
            id="faq-heading"
            className="font-display mt-2 text-3xl font-semibold tracking-tight md:text-4xl"
          >
            Everything about GaussMeridian
          </h2>
        </div>

        <div className="mt-10">
          {faqs.map((item) => (
            <details key={item.q} className="group border-border border-b py-4">
              <summary className="flex cursor-pointer list-none items-center justify-between gap-4 text-left font-medium [&::-webkit-details-marker]:hidden">
                {item.q}
                <ChevronDown
                  className="text-muted-foreground h-4 w-4 shrink-0 transition-transform group-open:rotate-180"
                  aria-hidden="true"
                />
              </summary>
              <p className="text-muted-foreground mt-3 text-sm leading-relaxed">{item.a}</p>
            </details>
          ))}
        </div>
      </div>
    </section>
  );
}
