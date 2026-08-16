"use client";

import { useState, type FormEvent } from "react";

import { siteConfig } from "@core/config";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";

export function Contact() {
  const [submitted, setSubmitted] = useState(false);

  function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    // Wire this to your CRM / backend lead endpoint. Presentational by default.
    setSubmitted(true);
  }

  return (
    <section id="contact" className="px-6 py-20">
      <div className="bg-brand-gradient shadow-glow relative mx-auto grid w-full max-w-6xl overflow-hidden rounded-3xl text-white lg:grid-cols-2">
        <div className="bg-grid absolute inset-0 opacity-20" />

        <div className="relative flex flex-col justify-between gap-10 p-10 sm:p-12">
          <div>
            <h2 className="font-display text-3xl font-semibold tracking-tight md:text-4xl">
              Get started today
            </h2>
            <p className="mt-3 max-w-sm text-white/80">
              Create a project, add one-time prepaid credit, and start routing through one stable
              Meridian contract. No recurring plan or automatic recharge.
            </p>
          </div>
          <ul className="flex flex-col gap-2 font-mono text-sm text-white/80">
            <li>✓ One-time organization credit</li>
            <li>✓ OpenAI-compatible API</li>
            <li>● {siteConfig.contact.email}</li>
          </ul>
        </div>

        <div className="bg-card text-card-foreground relative m-2 rounded-[1.35rem] p-8 sm:m-3">
          {submitted ? (
            <div
              role="status"
              className="flex h-full flex-col items-center justify-center gap-2 text-center"
            >
              <p className="font-display text-xl font-semibold">
                Thanks &mdash; we&apos;ll be in touch.
              </p>
              <p className="text-muted-foreground text-sm">
                Our team will reach out within one business day.
              </p>
            </div>
          ) : (
            <form onSubmit={handleSubmit} className="flex flex-col gap-4">
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="contact-name">Name</Label>
                <Input id="contact-name" name="name" required placeholder="Jane Doe" />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="contact-email">Work email</Label>
                <Input
                  id="contact-email"
                  name="email"
                  type="email"
                  required
                  placeholder="jane@company.com"
                />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="contact-company">Company</Label>
                <Input id="contact-company" name="company" placeholder="Acme Corp" />
              </div>
              <div className="flex flex-col gap-1.5">
                <Label htmlFor="contact-message">Tell us about your use case</Label>
                <Textarea
                  id="contact-message"
                  name="message"
                  placeholder="Describe your LLM routing needs, current providers, volume, and any questions..."
                />
              </div>
              <Button type="submit" variant="accent" size="lg">
                Get started
              </Button>
            </form>
          )}
        </div>
      </div>
    </section>
  );
}
