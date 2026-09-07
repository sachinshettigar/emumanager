import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "react-router-dom";
import { router } from "./router";
import "./index.css";

const rootEl = document.getElementById("root");
if (!rootEl) {
  throw new Error("Emulator Studio: #root element not found in index.html");
}

// One client for the app. Commands are explicit user actions or cheap probes,
// so don't retry failures or refetch on window focus by default.
const queryClient = new QueryClient({
  defaultOptions: {
    queries: { retry: false, refetchOnWindowFocus: false },
  },
});

createRoot(rootEl).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>,
);
