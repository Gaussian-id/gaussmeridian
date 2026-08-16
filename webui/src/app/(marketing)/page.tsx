import { Doors } from "@/components/sections/doors/doors";
import { HeadersSection } from "@/components/sections/headers/headers";
import { Hero } from "@/components/sections/hero";
import { HomeCta } from "@/components/sections/home-cta/home-cta";
import { Pipeline } from "@/components/sections/pipeline/pipeline";

export default function HomePage() {
  return (
    <>
      <Hero />
      <Pipeline />
      <HeadersSection />
      <Doors />
      <HomeCta />
    </>
  );
}
