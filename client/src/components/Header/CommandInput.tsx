import { useRef, useState, useCallback, useEffect } from "react";
import { Input } from "@/components/ui/input";
import { ChevronRight } from "lucide-react";
import useCommandSearch, { CommandAction, FunctionObject } from "./commandSearch";
import fuzzysort from "fuzzysort";
import { useNavigate } from "@tanstack/react-router";
import { db } from "@/db";

export default function CommandInput() {
  const [focused, setFocused] = useState(false);
  const [command, setCommand] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0); // Tracks selected result index
  const inputRef = useRef<HTMLInputElement>(null);
  const navigate = useNavigate();

  // Reset selection when command changes
  useEffect(() => {
    setSelectedIndex(0);
  }, [command]);

  const availableFunctions: FunctionObject[] = [
    { name: fuzzysort.prepare("MDM"), description: "Order Book", type: "function", for: "id" },
    { name: fuzzysort.prepare("G"), description: "Charts", type: "function", for: "id" },
  ];

  const { searchFn } = useCommandSearch(availableFunctions, { id: availableFunctions, function: [], none: [] });
  const search_results = searchFn(command);

  // Handle keyboard navigation
  const handleKeyNavigation = (evt: React.KeyboardEvent<HTMLDivElement>) => {
    switch (evt.code) {
      case "ArrowDown":
        evt.preventDefault();
        setSelectedIndex((prev) => (prev < search_results.length - 1 ? prev + 1 : 0));
        break;
      case "ArrowUp":
        evt.preventDefault();
        setSelectedIndex((prev) => (prev > 0 ? prev - 1 : search_results.length - 1));
        break;
      case "Tab":
        evt.preventDefault();
        break;
      case "Enter":
        if ((selectedIndex >= 0 && selectedIndex < search_results.length) || search_results.length > 0) {
          const result = search_results[selectedIndex >= 0 ? selectedIndex : 0].obj;
          evt.preventDefault();
          handleAction(result);
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
  };

  // ui garbage
  const focusInput = () => {
    if (inputRef.current) {
      inputRef.current.focus();
    }
  };

  const blurInput = () => {
    if (inputRef.current) {
      inputRef.current.blur();
    }
  };

  const handleFocus = () => setFocused(true);
  const handleBlur = () => setFocused(false);

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

  async function handleAction(action: CommandAction) {
    if (action.type) {
      switch (action.type) {
        case "id":
          setCommand((action.name as Fuzzysort.Prepared).target + " ");
          setSelectedIndex(0);
          break;
        case "function":
          const itemName = command.substring(0, command.lastIndexOf(" "));
          const id = await db.itemNames.where("name").equalsIgnoreCase(itemName).first();
          if (id) {
            setCommand(itemName + " ");
            blurInput();
            switch ((action.name as Fuzzysort.Prepared).target) {
              case "MDM":
                navigate({ to: `/market/${id.id}/mdm` });
                break;
              case "G":
                navigate({ to: `/market/${id.id}/g` });
                break;
            }
          }
          break;
      }
    }
  }

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
            <CommandBox
              name={result.target}
              action={result.obj}
              key={("id" in result.obj ? result.obj.id : index) + result.obj.description}
              selected={index === selectedIndex}
              onClick={(action: CommandAction) => handleAction(action)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function CommandBox({
  name,
  action,
  onClick,
  selected,
}: {
  name: string;
  action: CommandAction;
  onClick: (action: CommandAction) => void;
  selected?: boolean;
}) {
  return (
    <div className={`flex hover:bg-white/10 px-1 ${selected ? "bg-white/20" : ""}`} onClick={() => onClick(action)}>
      <p className="text-orange-500">{name}</p>
      <p className="ml-4 mr-2 font-main">{action.description}</p>
    </div>
  );
}
