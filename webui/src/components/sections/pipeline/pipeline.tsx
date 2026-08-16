import { PipelineScrollytelling, type PipelineStage } from "@/components/motion";

/** The customer-visible lifecycle of one funded request. */
const stages: PipelineStage[] = [
  {
    num: "01",
    short: "Key",
    name: "Authenticate the project",
    desc: "A reveal-once Meridian key identifies the exact project and organization making the request.",
    signalLabel: "scope",
    signal: "project: support-copilot",
  },
  {
    num: "02",
    short: "Credit",
    name: "Authorize prepaid credit",
    desc: "Meridian checks the organization wallet before model work begins, so unfunded traffic fails before supplier spend.",
    signalLabel: "wallet",
    signal: "credit: authorized",
  },
  {
    num: "03",
    short: "Model",
    name: "Validate the model request",
    desc: "The requested model and parameters are checked against the catalog Meridian currently supports.",
    signalLabel: "model",
    signal: "google/gemini-2.5-flash",
  },
  {
    num: "04",
    short: "Run",
    name: "Run the supported model",
    desc: "Meridian operates the upstream connection while your application keeps one provider-neutral API contract.",
    signalLabel: "status",
    signal: "inference: complete",
  },
  {
    num: "05",
    short: "Reply",
    name: "Normalize the response",
    desc: "The answer, finish state, and token usage return in the same Meridian response shape your application expects.",
    signalLabel: "object",
    signal: "chat.completion",
  },
  {
    num: "06",
    short: "Record",
    name: "Record exact usage",
    desc: "Measured tokens and provider cost become one auditable project record without inventing a retail wallet debit before that policy exists.",
    signalLabel: "recorded",
    signal: "958 tokens · settled",
  },
  {
    num: "07",
    short: "Trace",
    name: "Keep the evidence",
    desc: "Usage, provider cost, request identity, and paid-access status stay available in the console for the project team.",
    signalLabel: "visible",
    signal: "activity + billing",
  },
];

/** Home's folded-in deep-dive: how a prompt actually travels the pipeline. */
export function Pipeline() {
  return (
    <div id="how-it-works">
      <PipelineScrollytelling
        eyebrow="the contract"
        heading="One request. Seven accountable steps."
        stages={stages}
      />
    </div>
  );
}
