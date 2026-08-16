"use client";

import { gsap } from "gsap";
import { useEffect, useRef, useState } from "react";

import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useSaveSurvey } from "@/hooks/useOnboarding";

import { FloatingChoices, type FloatingChoiceItem } from "./floating-choices";
import { Prompt } from "./prompt";

// Card-less floating list styling for text inputs on the details screen — a transparent
// underline field, not a boxed input, so the Earth still reads through behind the survey.
const FLOATING_INPUT =
  "bg-transparent border-0 border-b border-white/25 rounded-none px-0 h-11 text-base text-white placeholder:text-white/35 transition-colors focus-visible:ring-0 focus-visible:ring-offset-0 focus-visible:border-accent aria-invalid:border-destructive";

// Emoji icons (from the approved prototype) give the survey its warmth — the plain, icon-less
// options were part of what made it read like a form rather than a conversation.
const INTEREST_OPTIONS: FloatingChoiceItem[] = [
  { value: "Cost savings", label: "Cost savings", icon: "💸" },
  { value: "Routing quality / reliability", label: "Routing quality", icon: "🎯" },
  { value: "Multi-model orchestration", label: "Multi-model / MoA", icon: "🧠" },
  { value: "Compliance / governance", label: "Compliance", icon: "🛡️" },
  { value: "Just exploring", label: "Just exploring", icon: "🧭" },
];

const TEAM_SIZE_OPTIONS: FloatingChoiceItem[] = [
  { value: "Just me", label: "Just me" },
  { value: "2-10", label: "2–10" },
  { value: "11-50", label: "11–50" },
  { value: "51-200", label: "51–200" },
  { value: "200+", label: "200+" },
];

/** The survey's internal conversation — one question per screen, invisible to the main rail. */
type SurveyPart = "interest" | "team" | "details";

interface OnboardingStepSurveyProps {
  onNext: () => void;
}

/**
 * Step 2 (US O2) — "About you." Restored to a **conversational, one-question-at-a-time** flow
 * (Shelby, PRD-22 follow-up): the prior single screen stacked interest + team + role + referral
 * into a dense form, losing the prototype's tap-through feel. Now it walks three sub-screens —
 * interest → team → details — each rendered through `<Prompt>` exactly like every other step, so
 * the survey reads as part of the same conversation rather than a questionnaire dropped into it.
 *
 * The sub-steps are internal: the main progress rail stays on "About you" throughout (they are
 * not onboarding steps), and the same four field names sent to `useSaveSurvey` — `role_title`,
 * `team_size`, `primary_interest`, `referral` — are unchanged, so no answer is lost. The step
 * itself is now **required** (Shelby, onboarding-refinement) — there's no skip; the user walks all
 * three screens and `Continue` on the last saves the answers. Interest, team size, and role/title
 * are required in this UI; only referral is optional.
 */
export function OnboardingStepSurvey({ onNext }: OnboardingStepSurveyProps) {
  const saveSurvey = useSaveSurvey();
  const [part, setPart] = useState<SurveyPart>("interest");
  const [primaryInterest, setPrimaryInterest] = useState<string | null>(null);
  const [teamSize, setTeamSize] = useState<string | null>(null);
  const [roleTitle, setRoleTitle] = useState("");
  const [roleTouched, setRoleTouched] = useState(false);
  const [referral, setReferral] = useState("");
  const trimmedRole = roleTitle.trim();
  const roleInvalid = roleTouched && !trimmedRole;

  // One question at a time — each sub-question floats in (gsap fade/rise/de-blur) and takes focus,
  // the way ConversationalStage does between main steps. The stage's own transition keys off the
  // main `currentStep`, which never changes across these internal parts, so the survey owns the
  // motion + focus here. The first render is skipped: the stage already animated and focused the
  // heading on entry to the step. `fromTo` (explicit end state) keeps the block discoverable
  // throughout, and prefers-reduced-motion drops the motion while still moving focus.
  const partWrapRef = useRef<HTMLDivElement>(null);
  const mountedRef = useRef(false);
  useEffect(() => {
    const wrap = partWrapRef.current;
    const heading = wrap?.querySelector<HTMLElement>("h1");
    const isFirstRender = !mountedRef.current;
    mountedRef.current = true;
    if (isFirstRender) return;

    const reducedMotion =
      typeof window.matchMedia === "function" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reducedMotion || !wrap) {
      heading?.focus({ preventScroll: true });
      return;
    }

    const ctx = gsap.context(() => {
      gsap.fromTo(
        wrap,
        { opacity: 0, y: 12, filter: "blur(4px)" },
        { opacity: 1, y: 0, filter: "blur(0px)", duration: 0.5, ease: "power3.out" },
      );
    }, wrap);
    const focusTimer = window.setTimeout(() => heading?.focus({ preventScroll: true }), 60);

    return () => {
      ctx.revert();
      window.clearTimeout(focusTimer);
    };
  }, [part]);

  function submit() {
    if (!primaryInterest || !teamSize || !trimmedRole) return;
    saveSurvey.mutate(
      {
        role_title: trimmedRole,
        team_size: teamSize,
        primary_interest: primaryInterest,
        referral: referral.trim() || undefined,
      },
      { onSuccess: onNext },
    );
  }

  return (
    <div ref={partWrapRef}>
      {part === "interest" && (
        <Prompt
          kicker="About you · 1 of 3"
          title="What brings you to Meridian?"
          description="Choose one to tailor your first dashboard."
          onContinue={() => setPart("team")}
          continueDisabled={!primaryInterest}
          guidance="Choose one option to continue."
        >
          <FloatingChoices
            options={INTEREST_OPTIONS}
            value={primaryInterest}
            onChange={setPrimaryInterest}
            ariaLabel="Primary interest"
            required
          />
        </Prompt>
      )}

      {part === "team" && (
        <Prompt
          kicker="About you · 2 of 3"
          title="How big is your team?"
          description="Choose one so we can set sensible defaults for limits and seats."
          onContinue={() => setPart("details")}
          continueDisabled={!teamSize}
          guidance="Choose your team size to continue."
        >
          <FloatingChoices
            options={TEAM_SIZE_OPTIONS}
            value={teamSize}
            onChange={setTeamSize}
            ariaLabel="Team size"
            required
          />
        </Prompt>
      )}

      {part === "details" && (
        <Prompt
          kicker="About you · 3 of 3"
          title="One last thing."
          description="Your role helps us tailor examples. Referral is optional."
          onContinue={submit}
          isBusy={saveSurvey.isPending}
          continueDisabled={!trimmedRole}
          continueDisabledReason={roleInvalid ? undefined : "Enter your role to continue."}
          continueLabel={saveSurvey.isPending ? "Saving…" : "Continue"}
          error={saveSurvey.isError ? "Could not save your answers. Please try again." : undefined}
        >
          <div className="flex flex-col gap-5">
            <div className="flex flex-col gap-1.5">
              <Label
                htmlFor="survey-role"
                className="text-xs font-medium tracking-wide text-white/70"
              >
                Role / title <span className="text-accent">Required</span>
              </Label>
              <Input
                id="survey-role"
                value={roleTitle}
                onChange={(event) => setRoleTitle(event.target.value)}
                onBlur={() => setRoleTouched(true)}
                placeholder="Platform engineer"
                required
                aria-invalid={roleInvalid || undefined}
                aria-describedby={roleInvalid ? "survey-role-error" : undefined}
                autoComplete="organization-title"
                disabled={saveSurvey.isPending}
                className={FLOATING_INPUT}
              />
              {roleInvalid && (
                <p id="survey-role-error" role="alert" className="text-destructive text-sm">
                  Enter your role to continue.
                </p>
              )}
            </div>
            <div className="flex flex-col gap-1.5">
              <Label
                htmlFor="survey-referral"
                className="text-xs font-medium tracking-wide text-white/70"
              >
                How did you hear about us? <span className="text-white/50">(optional)</span>
              </Label>
              <Input
                id="survey-referral"
                value={referral}
                onChange={(event) => setReferral(event.target.value)}
                placeholder="A friend, search, X…"
                disabled={saveSurvey.isPending}
                className={FLOATING_INPUT}
              />
            </div>
          </div>
        </Prompt>
      )}
    </div>
  );
}
