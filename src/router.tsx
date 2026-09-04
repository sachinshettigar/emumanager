import { createBrowserRouter } from "react-router-dom";
import { App } from "./App";
import { Dashboard } from "./routes/Dashboard";
import { Create } from "./routes/Create";
import { Dependencies } from "./routes/Dependencies";
import { Profiles } from "./routes/Profiles";

export const routeChildren = [
  { index: true, element: <Dashboard /> },
  { path: "create", element: <Create /> },
  { path: "dependencies", element: <Dependencies /> },
  { path: "profiles", element: <Profiles /> },
] as const;

/** Opt in to React Router v7 behavior now to keep console output clean. */
export const routerFuture = {
  v7_startTransition: true,
  v7_relativeSplatPath: true,
  v7_fetcherPersist: true,
  v7_normalizeFormMethod: true,
  v7_partialHydration: true,
  v7_skipActionErrorRevalidation: true,
} as const;

export const router = createBrowserRouter(
  [
    {
      path: "/",
      element: <App />,
      children: [...routeChildren],
    },
  ],
  { future: routerFuture },
);
