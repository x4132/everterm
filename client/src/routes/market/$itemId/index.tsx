import InvalidID from "@/components/InvalidID";
import { useQuery } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/market/$itemId/")({
  loader: async ({ params }) => {
    return parseInt(params.itemId);
  },
  component: RouteComponent,
});

function RouteComponent() {
  const itemId = Route.useLoaderData();

  if (isNaN(itemId)) {
    return <InvalidID />;
  }

  const orders = useQuery({
    queryKey: ["orderbook", itemId],
    queryFn: async () => {
      const response = await fetch(`/api/market/orders/${itemId}`);
      if (!response.ok) {
        throw new Error(response.statusText);
      }

      return response.json();
    },
  });

  return (
    <div>
      <div>Hello "/market/{itemId}/"!</div>
      <div>{orders.data}</div>
    </div>
  );
}
