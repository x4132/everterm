import { createFileRoute, Navigate } from "@tanstack/react-router";

export const Route = createFileRoute("/market/")({
  component: RouteComponent,
});

function RouteComponent() {
  return <Navigate to="/market/$itemId" params={{ itemId: "44992" }} />;
}
