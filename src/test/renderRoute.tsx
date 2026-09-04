import { render, type RenderResult } from "@testing-library/react";
import { createMemoryRouter, RouterProvider } from "react-router-dom";
import { App } from "../App";
import { routeChildren, routerFuture } from "../router";

/** Render the full app shell at a given path using an in-memory router. */
export function renderRoute(path: string): RenderResult {
  const router = createMemoryRouter(
    [{ path: "/", element: <App />, children: [...routeChildren] }],
    { initialEntries: [path], future: routerFuture },
  );
  return render(<RouterProvider router={router} />);
}
