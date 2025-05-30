import Dexie, { type EntityTable } from "dexie";
import { z } from "zod/v4-mini";

export const MarketGroup = z.object({
  name: z.string(),
  description: z.string(),
  market_group_id: z.number(),
  parent_group_id: z.optional(z.number()),
  types: z.array(z.number())
});

export type MarketGroup = z.infer<typeof MarketGroup>;

export const UniverseName = z.object({
  id: z.number(),
  name: z.string(),
  category: z.optional(z.string())
});

export type UniverseName = z.infer<typeof UniverseName>;

export const db = new Dexie("esi-db") as Dexie & {
  marketGroups: EntityTable<MarketGroup, "market_group_id">,
  itemNames: EntityTable<UniverseName, "id">
};
db.version(1).stores({
  marketGroups: "++market_group_id, name, description, parent_group_id",
  itemNames: "++id, name, category"
});
