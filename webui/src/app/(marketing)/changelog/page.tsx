import { ChangelogHero, ChangelogTimeline } from "@/components/sections/changelog/changelog";
import { changelogEntries } from "@/lib/changelog/entries";

export default function ChangelogPage() {
  return (
    <>
      <ChangelogHero />
      <ChangelogTimeline entries={changelogEntries} />
    </>
  );
}
