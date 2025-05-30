import { useQuery } from "@tanstack/react-query";
import fuzzysort from "fuzzysort";
import { item_names } from "@/lib/staticESIQueries";
import { useCallback } from "react";

export default function useCommandSearch() {
    const { data } = useQuery({
        queryKey: ["item_names"],
        queryFn: async () =>
            Array.from(await item_names()).map((tuple) => ({
                name: fuzzysort.prepare(tuple[1]),
                id: tuple[0],
                to: `/market/${tuple[0]}/`
            })),
    });

    const searchFn = useCallback(
        (command: string) =>
            data ? fuzzysort.go(command, data, {
                threshold: -10000,
                limit: 5,
                key: "name"
            }) : [],
        [data],
    );

    return { searchFn };
}