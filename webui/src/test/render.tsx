import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render } from "@testing-library/react";

import { AdapterProvider, type AdapterRegistry } from "@core/adapters";

import type { ReactElement, ReactNode } from "react";

/** Render a component inside the query client + injected adapter registry (the seam). */
export function renderWithProviders(ui: ReactElement, registry: AdapterRegistry) {
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });

  function Wrapper({ children }: { children: ReactNode }) {
    return (
      <QueryClientProvider client={queryClient}>
        <AdapterProvider registry={registry}>{children}</AdapterProvider>
      </QueryClientProvider>
    );
  }

  return render(ui, { wrapper: Wrapper });
}
