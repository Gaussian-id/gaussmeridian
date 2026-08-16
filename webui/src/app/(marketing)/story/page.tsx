import { CtaBand } from "@/components/sections/cta-band/cta-band";
import {
  StoryCompany,
  StoryHero,
  StoryName,
  StoryPrinciples,
  StoryProblem,
} from "@/components/sections/story/story";

export default function StoryPage() {
  return (
    <>
      <StoryHero />
      <StoryProblem />
      <StoryName />
      <StoryPrinciples />
      <StoryCompany />
      <CtaBand
        title="Find your line."
        subtitle="One project key, a supported model catalog, and auditable usage tied to one funded organization."
        primary={{ label: "Get API key →", href: "/signup" }}
        secondary={{ label: "See the platform", href: "/" }}
      />
    </>
  );
}
