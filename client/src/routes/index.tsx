import { createFileRoute } from '@tanstack/react-router'

export const Route = createFileRoute('/')({
  component: App,
})

function App() {
  return (
    <div className="dark bg-background text-primary h-full w-full">
      <h1 className="text-4xl" >Everterm</h1>
    </div>
  )
}
