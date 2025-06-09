import { z } from "zod/v4-mini";

export const MarketOrder = z.object({
  id: z.number(),
  is_buy_order: z.boolean(),
  price: z.number(),
  expiry: z.coerce.date(),
  issued: z.coerce.date(),
  location_id: z.number(),
  min_volume: z.number(),
  range: z.union([z.string(), z.object({ System: z.number() })]),
  system_id: z.number(),
  volume_remain: z.number(),
  volume_total: z.number(),
});
export type MarketOrder = z.infer<typeof MarketOrder>;

export const MarketOrderBook = z.array(MarketOrder);
export type MarketOrderBook = z.infer<typeof MarketOrderBook>;

export const Station = z.object({
  id: z.number(),
  system_id: z.number(),
  name: z.string(),
  type_id: z.optional(z.number()),
});
export type Station = z.infer<typeof Station>;

export const RefreshIntervals = z.record(z.string(), z.coerce.date());
export type RefreshIntervals = z.infer<typeof RefreshIntervals>;

export const Region = z.object({
  constellations: z.array(z.number()),
  description: z.string(),
  name: z.string(),
  region_id: z.number(),
});
export type Region = z.infer<typeof Region>;