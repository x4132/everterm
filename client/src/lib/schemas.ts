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
