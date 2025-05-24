import { useRef, useState, useCallback } from "react";
import { Input } from "./ui/input";
import { ChevronRight } from "lucide-react";

export default function CommandInput() {
  const [focused, setFocused] = useState(false);
  const [command, setCommand] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  const focusInput = useCallback(() => {
    if (inputRef.current) {
      inputRef.current.focus();
    }
  }, []);

  const blurInput = useCallback(() => {
    if (inputRef.current) {
      inputRef.current.blur();
    }
  }, []);

  const handleFocus = useCallback(() => setFocused(true), []);
  const handleBlur = useCallback(() => setFocused(false), []);

  const handleShortcuts = useCallback((evt: React.KeyboardEvent<HTMLDivElement>) => {
    switch (evt.code) {
      case "Escape":
        if (command === "") {
          blurInput();
        } else {
          setCommand("");
        }
        break;
    }
  }, [command, focusInput, blurInput]);

  return (
    <div className="flex grow items-center relative">
      <div
        className="w-full flex text-2xl items-center bg-input focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]"
        onKeyDown={handleShortcuts}
      >
        <ChevronRight className="cursor-text" onClick={focusInput} />
        <Input
          className="rounded-none px-1 py-0 m-0 text-2xl uppercase font-mono focus-visible:border-none focus-visible:ring-0"
          onChange={(evt) => setCommand(evt.target.value)}
          value={command}
          onFocus={handleFocus}
          onBlur={handleBlur}
          ref={inputRef}
        />
      </div>

      <div className={`absolute top-full border w-[640px] flex flex-col ${focused ? "block" : "hidden"}`}>
        <div>Blank Input</div>
      </div>
    </div>
  );
}