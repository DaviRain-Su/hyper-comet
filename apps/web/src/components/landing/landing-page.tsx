import { SiteNav } from "@/components/landing/site-nav";
import { Hero } from "@/components/landing/hero";
import { ShowcaseSection, DownloadSection } from "@/components/landing/showcase";
import {
  ProductSection,
  DiffSection,
  WorkflowSection,
  ProofForgeSection,
  PricingSection,
  HonestySection,
  FinalCta,
  SiteFooter,
} from "@/components/landing/sections";

export function LandingPage() {
  return (
    <div className="min-h-dvh bg-bg text-fg">
      <SiteNav />
      <main>
        <Hero />
        <ShowcaseSection />
        <ProductSection />
        <DiffSection />
        <WorkflowSection />
        <DownloadSection />
        <ProofForgeSection />
        <PricingSection />
        <HonestySection />
        <FinalCta />
      </main>
      <SiteFooter />
    </div>
  );
}
