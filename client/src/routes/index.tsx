import { createFileRoute } from "@tanstack/react-router";

export const Route = createFileRoute("/")({
  component: App,
});

function App() {
  return (
    <div className="grow dark bg-background text-primary w-full flex flex-col justify-center items-center">
      <h1 className="h1 text-8xl">Everterm</h1>
      <h2 className="h2">Version TODO: add version info</h2>
    </div>
  );
}
