import Dexie, { type EntityTable } from "dexie";
import { z } from "zod/v4-mini";
import type { Region } from "./lib/schemas";

export const UniverseType = z.object({
  capacity: z.optional(z.number()),
  description: z.string(),
  dogma_attributes: z.optional(
    z.array(
      z.object({
        attribute_id: z.number(),
        value: z.number(),
      }),
    ),
  ),
  dogma_effects: z.optional(
    z.array(
      z.object({
        effect_id: z.number(),
        is_default: z.boolean(),
      }),
    ),
  ),
  graphic_id: z.optional(z.number()),
  group_id: z.number(),
  icon_id: z.optional(z.number()),
  market_group_id: z.optional(z.number()),
  mass: z.optional(z.number()),
  name: z.string(),
  packaged_volume: z.optional(z.number()),
  portion_size: z.optional(z.number()),
  published: z.boolean(),
  radius: z.optional(z.number()),
  type_id: z.number(),
  volume: z.optional(z.number()),
});
export type UniverseType = z.infer<typeof UniverseType>;

export const MarketGroup = z.object({
  name: z.string(),
  description: z.string(),
  market_group_id: z.number(),
  parent_group_id: z.optional(z.number()),
  types: z.array(z.number()),
});

export type MarketGroup = z.infer<typeof MarketGroup>;

export const UniverseName = z.object({
  id: z.number(),
  name: z.string(),
  category: z.optional(z.string()),
});

export type UniverseName = z.infer<typeof UniverseName>;

export const db = new Dexie("esi-db") as Dexie & {
  marketGroups: EntityTable<MarketGroup, "market_group_id">;
  itemNames: EntityTable<UniverseName, "id">;
  regionNames: EntityTable<Region, "region_id">;
};
db.version(1).stores({
  marketGroups: "++market_group_id, name, description, parent_group_id",
  itemNames: "++id, name, category",
  regionNames: "++region_id, name, description, constellations",
});
