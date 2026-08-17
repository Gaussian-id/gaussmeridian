"use client";

import Link from "next/link";
import { useState } from "react";

import { Reveal } from "@/components/motion";
import { buttonVariants } from "@/components/ui/button";
import { useChat } from "@/hooks/useChat";
import { useModels } from "@/hooks/useGaussmeridianQueries";

import { PlaygroundComposer } from "./playground-composer";
import { PlaygroundHeader } from "./playground-header";
import { PlaygroundSidebar } from "./playground-sidebar";
import { PlaygroundThreadView } from "./playground-thread";

interface PlaygroundProps {
  projectId?: string;
  orgId?: string;
}

/** Authenticated chat surface for one explicit project. The global entry point is retained as a
 * project chooser; inference controls are never rendered until the URL carries canonical project
 * context through the project-scoped route. */
export function Playground({ projectId, orgId }: PlaygroundProps) {
  if (!projectId) {
    return (
      <PlaygroundState
        title="Choose a project"
        description="The Playground records usage and credit against one project. Open a project before sending a prompt."
        action={
          <Link href="/orgs" className={buttonVariants({ variant: "accent", size: "sm" })}>
            View organizations
          </Link>
        }
      />
    );
  }

  return <ProjectPlayground projectId={projectId} orgId={orgId} />;
}

function ProjectPlayground({ projectId, orgId }: { projectId: string; orgId?: string }) {
  const { threads, activeThread, isStreaming, send, newChat, selectThread } = useChat(projectId);
  const [requestedModel, setRequestedModel] = useState("");
  const models = useModels();

  if (models.isLoading) {
    return (
      <PlaygroundState
        title="Loading available models"
        description="GaussMeridian is loading the enabled model catalog for this project."
        status
      />
    );
  }

  if (models.isError || !models.data) {
    return (
      <PlaygroundState
        title="Could not load the model catalog"
        description="The Playground cannot send a request until the enabled catalog is available."
        action={
          <button
            type="button"
            className={buttonVariants({ variant: "outline", size: "sm" })}
            onClick={() => void models.refetch()}
          >
            Try again
          </button>
        }
      />
    );
  }

  if (models.data.data.length === 0) {
    return (
      <PlaygroundState
        title="No models are enabled for this project"
        description="Ask an administrator to enable a GaussMeridian model before using the Playground."
      />
    );
  }

  const selectedModel = models.data.data.some((entry) => entry.id === requestedModel)
    ? requestedModel
    : models.data.data[0].id;
  const needsCredit = activeThread.messages.at(-1)?.recovery === "add-credit";

  function handleSend(text: string) {
    void send(text, { model: selectedModel });
  }

  return (
    <Reveal>
      <div className="border-border bg-card flex h-[calc(100dvh-8rem)] overflow-hidden rounded-xl border shadow-sm">
        <PlaygroundSidebar
          threads={threads}
          activeThreadId={activeThread.id}
          onSelectThread={selectThread}
          onNewChat={newChat}
        />
        <div className="flex min-w-0 flex-1 flex-col">
          <PlaygroundHeader
            model={selectedModel}
            models={models.data.data}
            onModelChange={setRequestedModel}
          />
          <PlaygroundThreadView thread={activeThread} />
          {needsCredit && orgId ? (
            <div
              className="border-border bg-secondary/45 mx-4 mb-3 flex flex-col gap-3 rounded-lg border px-4 py-3 text-sm sm:mx-6 sm:flex-row sm:items-center sm:justify-between"
              role="alert"
            >
              <div>
                <p className="font-semibold">Prepaid credit required</p>
                <p className="text-muted-foreground mt-0.5">
                  Add credit to this organization, then retry the same request.
                </p>
              </div>
            </div>
          ) : null}
          <PlaygroundComposer onSend={handleSend} disabled={isStreaming} />
        </div>
      </div>
    </Reveal>
  );
}

function PlaygroundState({
  title,
  description,
  action,
  status = false,
}: {
  title: string;
  description: string;
  action?: React.ReactNode;
  status?: boolean;
}) {
  return (
    <Reveal>
      <section
        className="border-border bg-card flex min-h-80 flex-col items-center justify-center gap-3 rounded-xl border px-6 text-center shadow-sm"
        role={status ? "status" : undefined}
        aria-live={status ? "polite" : undefined}
      >
        <h1 className="font-display text-2xl font-semibold tracking-tight">{title}</h1>
        <p className="text-muted-foreground max-w-lg text-sm">{description}</p>
        {action}
      </section>
    </Reveal>
  );
}
