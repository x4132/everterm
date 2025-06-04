import { createFileRoute, redirect } from '@tanstack/react-router'

export const Route = createFileRoute('/market/$itemId/')({
  beforeLoad: ({ params }) => {
    throw redirect({
      to: '/market/$itemId/mdm',
      params: { itemId: params.itemId }
    })
  },
  component: RouteComponent,
})

function RouteComponent() {
  return <div>Hello "/market/$itemId/"!</div>
}
