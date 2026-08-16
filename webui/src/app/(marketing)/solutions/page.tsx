import {
  ProofCode,
  SolutionScene,
  SolutionsCta,
  SolutionsIntro,
  TrustGrid,
} from "@/components/sections/solutions/solutions";

const devLines = [
  <>
    <span className="text-[#bcd4ff]">from</span> openai{" "}
    <span className="text-[#bcd4ff]">import</span> OpenAI
  </>,
  <>&nbsp;</>,
  <>client = OpenAI(</>,
  <>
    &nbsp;&nbsp;base_url=
    <span className="text-[#9fe6c4]">&quot;https://api.gaussian.id/v1&quot;</span>,
  </>,
  <>
    &nbsp;&nbsp;api_key=<span className="text-[#9fe6c4]">&quot;mrd-…&quot;</span>)
  </>,
  <>&nbsp;</>,
  <>client.chat.completions.create(</>,
  <>
    &nbsp;&nbsp;model=<span className="text-[#9fe6c4]">&quot;google/gemini-2.5-flash&quot;</span>,
  </>,
  <>&nbsp;&nbsp;messages=[…])</>,
];

const teamLines = [
  <>
    organization&nbsp;&nbsp;<span className="text-[#9fe6c4]">Gaussian Labs</span>
  </>,
  <>
    wallet&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
    <span className="text-[#9fe6c4]">Rp500.000</span>
  </>,
  <>
    project&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
    <span className="text-white">support-copilot</span>
  </>,
  <>&nbsp;</>,
  <>
    <span className="text-[#7ee0b8]">✓</span> project key authenticated
  </>,
  <>
    <span className="text-[#7ee0b8]">✓</span> supported model completed
  </>,
  <>
    <span className="text-[#7ee0b8]">✓</span> usage recorded to project
  </>,
  <>
    access&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;
    <span className="text-[#9fe6c4]">purchased credit verified</span>
  </>,
];

const trust = [
  { t: "SSO / SAML", s: "Okta, Entra, Google" },
  { t: "Role-based access", s: "Per-project scopes" },
  { t: "Budget caps", s: "Hard limits + alerts" },
  { t: "Audit log", s: "Every request, exportable" },
  { t: "Payment evidence", s: "Order, invoice, wallet" },
  { t: "SLAs & support", s: "Operational response" },
];

export default function SolutionsPage() {
  return (
    <>
      <SolutionsIntro />

      <SolutionScene
        id="developers"
        kicker="For developers"
        title="Build on it."
        desc="Point an OpenAI-compatible SDK at Meridian, authenticate with a project key, and call a model from the supported catalog through one stable contract."
        bullets={[
          "Drop-in OpenAI-compatible — change the base URL, nothing else.",
          "Choose an explicit model from the catalog returned for your account.",
          "Normalized responses include the model and token usage for the completed call.",
        ]}
        ctas={[
          { label: "Get your API key →", href: "/signup", brand: true },
          { label: "Read the quickstart", href: "/docs" },
        ]}
        proof={<ProofCode lines={devLines} />}
      />

      <SolutionScene
        id="teams"
        reversed
        kicker="For product teams"
        title="Fund one shared wallet."
        desc="Keep projects, keys, members, and prepaid model usage under the organization that owns the product. No recurring plan is required."
        bullets={[
          "Project-scoped keys keep application traffic separated.",
        ]}
        ctas={[
          { label: "Read the API guide", href: "/docs", brand: true },
        ]}
        proof={<ProofCode lines={teamLines} />}
      />

      <SolutionScene
        id="enterprise"
        kicker="For enterprise"
        title="Deploy at scale."
        desc="For organizations that need clear identity, permissions, project boundaries, payment evidence, and reliable metering around every model request."
        ctas={[
          { label: "Talk to us →", href: "/signup", brand: true },
          { label: "Book a demo", href: "/signup" },
        ]}
        proof={<TrustGrid cells={trust} />}
      />

      <SolutionsCta />
    </>
  );
}
