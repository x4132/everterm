import CommandInput from "./CommandInput";
import Status from "./Status";

export default function Header() {
  return (
    <nav className="flex items-center dark bg-background text-primary space-x-2">
      <CommandInput />
      <Status />
    </nav>
  );
}

