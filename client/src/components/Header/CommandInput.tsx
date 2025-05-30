import { useRef, useState, useCallback, useEffect } from "react";
import { Input } from "@/components/ui/input";
import { ChevronRight } from "lucide-react";
import { Link, useNavigate } from "@tanstack/react-router";
import useCommandSearch from './useCommandSearch';

export default function CommandInput() {
  const [focused, setFocused] = useState(false);
  const [command, setCommand] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(-1); // Tracks selected result index
  const inputRef = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();

  // Reset selection when command changes
  useEffect(() => {
    setSelectedIndex(-1);
  }, [command]);

  const { searchFn } = useCommandSearch();
  const search_results = searchFn(command) ?? [];

  // Handle keyboard navigation
  const handleKeyNavigation = useCallback(
    (evt: React.KeyboardEvent<HTMLDivElement>) => {
      switch (evt.code) {
        case "ArrowDown":
          evt.preventDefault();
          setSelectedIndex((prev) =>
            prev < search_results.length - 1 ? prev + 1 : 0
          );
          break;
        case "ArrowUp":
          evt.preventDefault();
          setSelectedIndex((prev) =>
            prev > 0 ? prev - 1 : search_results.length - 1
          );
          break;
        case "Tab":
          if (selectedIndex >= 0 && selectedIndex < search_results.length) {
            evt.preventDefault();
            setCommand(search_results[selectedIndex].target);
            setSelectedIndex(-1);
          }
          break;
        case "Enter":
          if (selectedIndex >= 0 && selectedIndex < search_results.length) {
            evt.preventDefault();
            navigate({ to: search_results[selectedIndex].obj.to });
            setCommand("");
            blurInput();
          } else if (search_results.length > 0) {
            // If no item selected but results exist, navigate to first result
            evt.preventDefault();
            navigate({ to: search_results[0].obj.to });
            setCommand("");
            blurInput();
          }
          break;
        case "Escape":
          if (command === "") {
            blurInput();
          } else {
            setCommand("");
          }
          break;
      }
    },
    [command, search_results, selectedIndex, navigate]
  );

  // ui garbage
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
    [inputRef],
  );

  useEffect(() => {
    window.addEventListener("keydown", globalShortcutHandler);

    return () => {
      window.removeEventListener("keydown", globalShortcutHandler);
    };
  }, [globalShortcutHandler]);

  return (
    <div className="flex grow items-center relative">
      <div
        className={`flex grow text-2xl items-center ${focused ? "bg-input" : "bg-transparent"} focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px] font-code`}
        onKeyDown={handleKeyNavigation}
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
        className={`absolute top-full border w-[640px] flex flex-col z-10 bg-background font-mono p-2 ${focused ? "block" : "hidden"}`}
        onMouseDown={(e) => e.preventDefault()}
      >
        <div>
          {search_results.map((result, index) => (
            <CommandAction
              name={result.target}
              to={result.obj.to}
              key={result.obj.id}
              selected={index === selectedIndex}
              onNavigate={() => {
                setCommand("");
                blurInput();
              }}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function CommandAction({ name, to, onNavigate, selected }: {
  name: string;
  to?: string;
  onNavigate?: () => void;
  selected?: boolean;
}) {
  return (
    <Link
      to={to}
      className={`block hover:bg-white/10 ${selected ? "bg-white/20" : ""}`}
      onClick={onNavigate}
    >
      {name}
    </Link>
  );
}
