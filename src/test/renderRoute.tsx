import { render, type RenderResult } from "@testing-library/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { App } from "../App";
import { routeChildren, routerFuture } from "../router";

/** Render the full app shell at a given path using an in-memory router. */
export function renderRoute(path: string): RenderResult {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  const router = createMemoryRouter(
    [{ path: "/", element: <App />, children: [...routeChildren] }],
    { initialEntries: [path], future: routerFuture },
  );
  return render(
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>,
  );
}
