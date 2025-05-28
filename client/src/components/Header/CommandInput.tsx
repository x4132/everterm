import { useRef, useState, useCallback, useEffect } from "react";
import { Input } from "@/components/ui/input";
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

  const globalShortcutHandler = useCallback(
    (evt: KeyboardEvent) => {
      if (inputRef.current && document.activeElement != inputRef.current) {
        switch (evt.code) {
          case "Backquote":
          case "Enter":
            inputRef.current.focus();
            evt.preventDefault();
            return false;
        }
      }
    },
    [inputRef.current],
  );

  useEffect(() => {
    window.addEventListener("keydown", globalShortcutHandler);

    return () => {
      window.removeEventListener("keydown", globalShortcutHandler);
    };
  }, [inputRef]);

  const localShortcutHandler = useCallback(
    (evt: React.KeyboardEvent<HTMLDivElement>) => {
      switch (evt.code) {
        case "Escape":
          if (command === "") {
            blurInput();
          } else {
            setCommand("");
          }
          break;
      }
    },
    [command, focusInput, blurInput],
  );

  return (
    <div className="flex grow items-center relative">
      <div
        className={`flex grow text-2xl items-center ${focused ? "bg-input" : "bg-transparent"} focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] font-code`}
        onKeyDown={localShortcutHandler}
      >
        <ChevronRight className="cursor-text" onClick={focusInput} />
        <Input
          className="rounded-none px-1 py-0 m-0 text-2xl uppercase focus-visible:border-none focus-visible:ring-0 border-0"
          onChange={(evt) => setCommand(evt.target.value)}
          value={command}
          onFocus={handleFocus}
          onBlur={handleBlur}
          ref={inputRef}
        />
      </div>

      <div
        className={`absolute top-full border w-[640px] flex flex-col z-10 bg-background p-2 ${focused ? "block" : "hidden"}`}
      >
        <div>Blank Input</div>
      </div>
    </div>
  );
}
