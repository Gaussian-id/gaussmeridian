import { CtaBand } from "@/components/sections/cta-band/cta-band";
import {
  OutcomeBilling,
  PricingFaq,
  PricingHero,
  PricingTiers,
} from "@/components/sections/pricing/pricing";

export default function PricingPage() {
  return (
    <>
      <PricingHero />
      <PricingTiers />
      <OutcomeBilling />
      <PricingFaq />
      <CtaBand
        title="Bring Meridian to your organization."
        subtitle="Custom terms, governance, deployment options, and support shaped around your requirements."
        primary={{ label: "Talk to sales", href: "/signup" }}
        secondary={{ label: "Explore solutions", href: "/solutions" }}
      />
    </>
  );
}
