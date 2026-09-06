import {
  createHashHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/solid-router";
import { Home } from "~/pages/Home";
import { TitleBar } from "~/components/TitleBar";

const rootRoute = createRootRoute({
  component: () => (
    <>
      <TitleBar />
      <Outlet />
    </>
  ),
});

const homeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: Home,
});

const routeTree = rootRoute.addChildren([homeRoute]);
const router = createRouter({ routeTree });

export function Router() {
  return <RouterProvider router={router} history={createHashHistory()} />;
}

declare module "@tanstack/solid-router" {
  interface Register {
    router: typeof router;
  }
}
