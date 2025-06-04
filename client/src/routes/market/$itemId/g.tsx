import InvalidID from '@/components/InvalidID';
import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/market/$itemId/g')({
  loader: async ({ params }) => {
    return parseInt(params.itemId);
  },
  component: RouteComponent,
})

function RouteComponent() {

  const itemId = Route.useLoaderData();
  if (isNaN(itemId)) {
    return <InvalidID />
  }
  return <div>Hello "/market/$itemId/graph"!</div>
}
