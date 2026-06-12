import {
  createRootRoute,
  createRoute,
  createRouter,
} from "@tanstack/react-router";
import { Layout } from "@/components/Layout";
import { DashboardPage } from "@/routes/dashboard";
import { SkillsListPage } from "@/routes/skills";
import { SkillsNewPage } from "@/routes/skills-new";
import { SkillDetailPage } from "@/routes/skill-detail";
import { OrgsPage } from "@/routes/orgs";
import { GrantsPage } from "@/routes/grants";
import { AuditPage } from "@/routes/audit";
import { TokensPage } from "@/routes/tokens";

const rootRoute = createRootRoute({ component: Layout });

const indexRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/",
  component: DashboardPage,
});

const skillsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/skills",
  component: SkillsListPage,
});

const skillsNewRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/skills/new",
  component: SkillsNewPage,
});

const skillsDetailRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/skills/$id",
  component: SkillDetailPage,
});

const orgsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/orgs",
  component: OrgsPage,
});

const grantsRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/grants",
  component: GrantsPage,
});

const auditRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/audit",
  component: AuditPage,
});

const tokensRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/tokens",
  component: TokensPage,
});

const routeTree = rootRoute.addChildren([
  indexRoute,
  skillsRoute,
  skillsNewRoute,
  skillsDetailRoute,
  orgsRoute,
  grantsRoute,
  auditRoute,
  tokensRoute,
]);

export const router = createRouter({ routeTree, defaultPreload: "intent" });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
