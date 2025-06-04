import { useQuery } from "@tanstack/react-query";
import fuzzysort from "fuzzysort";
import { item_names } from "@/lib/staticESIQueries";
import { useCallback, useRef } from "react";
import { z } from "zod";

export const ItemObject = z.object({
  type: z.literal("id"),
  id: z.number(),
  name: z.any(),
  description: z.string(),
  for: z.string().optional(),
});
export type ItemObject = z.infer<typeof ItemObject>;
export const FunctionObject = z.object({
  type: z.literal("function"),
  name: z.any(),
  description: z.string(),
  for: z.string().optional(),
});
export type FunctionObject = z.infer<typeof FunctionObject>;
export const CommandAction = z.union([ItemObject, FunctionObject]);
export type CommandAction = z.infer<typeof CommandAction>;

export default function useCommandSearch(
  availableFunctions: FunctionObject[],
  defaultResults: { id: CommandAction[]; function: CommandAction[]; none: CommandAction[] },
): { searchFn: (command: string) => Fuzzysort.KeyResults<CommandAction> } {
  const { data: item_name_data } = useQuery({
    queryKey: ["item_names"],
    queryFn: async () =>
      Array.from(await item_names()).map((tuple) =>
        ItemObject.parse({
          type: "id",
          name: fuzzysort.prepare(tuple[1][0]),
          description: `${tuple[1][1]} > ${tuple[1][0]}`,
          id: tuple[0],
          for: undefined,
        }),
      ),
  });

  const data: CommandAction[] = [item_name_data || [], availableFunctions].flat(1);
  const ids = new Set(
    data
      .filter((item) => item && item.type === "id")
      .map((item) => (item.name as Fuzzysort.Prepared).target.toLowerCase()),
  );

  const previousResultsRef = useRef<Fuzzysort.KeyResults<CommandAction>>(
    Object.assign([], { total: 0 }) as Fuzzysort.KeyResults<CommandAction>,
  );

  const searchFn = useCallback(
    (command: string): Fuzzysort.KeyResults<CommandAction> => {
      let spacePos = Math.max(command.length - 1, 0);
      let prevKey = undefined;

      // HACK: this is god awful and kills search performance, but this is way faster than any other server-based searchers so i dont care
      do {
        let substr = command.substring(0, spacePos);
        if (ids.has(substr.toLowerCase())) {
          command = command.substring(spacePos, command.length);
          prevKey = "id";
          break;
        }

        spacePos = command.lastIndexOf(" ", spacePos - 1);
      } while (spacePos !== -1);

      if (command.trim() === "") {
        switch (prevKey) {
          case "id":
            return fuzzysort.go("", defaultResults.id, { key: "name", all: true });
          case "function":
            return fuzzysort.go("", defaultResults.function, { key: "name", all: true });
          default:
            return fuzzysort.go("", defaultResults.none, { key: "name", all: true });
        }
      }

      let searchData = data;

      if (prevKey) {
        searchData = data.filter((item) => item.for === prevKey);
      } else {
        searchData = data.filter((item) => item.for === undefined);
      }

      if (searchData) {
        const results = fuzzysort.go(command, searchData, {
          threshold: -10000,
          limit: 5,
          key: "name",
        });
        previousResultsRef.current = results;
        return results;
      }
      return previousResultsRef.current;
    },
    [data],
  );

  return { searchFn };
}
