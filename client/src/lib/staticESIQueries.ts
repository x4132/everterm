import {db, MarketGroup, UniverseName} from "@/db";
import esi from "@/lib/esiClient";
import {z} from "zod/v4-mini"; // TODO: consider whether zod or zod mini is better

const MarketGroupList = z.array(z.number());

export async function market_group_names(): Promise<MarketGroup[]> {
  const ls_key = "ETAG_Pages-/markets/groups";
  const market_groups = await MarketGroupList.parseAsync(await esi.get("markets/groups").json());

  if (localStorage.getItem(ls_key)) {
    // TODO: Consider whether or not checking Etags for each item description is necessary
    // const ETagPageSchema = z.array(z.object({ id: z.number(), tag: z.string() }));
    // const parsed_etags = ETagPageSchema.safeParse(JSON.parse(localStorage.getItem(ls_key) ?? "{}"));

    // if (parsed_etags.success) {
    //   const etags = parsed_etags.data;

    //   const etag_promises: ResponsePromise<unknown>[] = [];
    //   etags.forEach((etag) => {
    //     etag_promises.push(
    //       esi_check.get(`markets/groups/${etag.id}/`, {
    //         headers: { "If-None-Match": etag.tag },
    //       }),
    //     );
    //   });

    //   const etag_results = await Promise.allSettled(etag_promises);

    //   if (
    //     etag_results.reduce(
    //       (prev, cur) => (prev ? (cur.status === "fulfilled" ? cur.value.status === 304 : false) : prev),
    //       true,
    //     )
    //   ) {

    //   }

    return db.marketGroups.toArray();
  }

  const group_promises = market_groups.map((group) => esi.get(`markets/groups/${group}/`));

  const groups = [];
  // we batch group requests into groups of 5 instead of all at once (this is 3600+ requests)
  for (let i = 0; i < group_promises.length; i++) {
    groups.push(...(await Promise.all(group_promises.slice(i, i + 5)))); // yeah i know this is cursed
  }

  const names = await Promise.all(
    groups.map((resp) => resp.json()),
  );

  await db.marketGroups.bulkAdd(z.array(MarketGroup).parse(names));

  localStorage.setItem(
    ls_key,
    JSON.stringify(
      groups.map((request, index) => ({
        id: market_groups[index],
        tag: request.headers.get("etag") ?? "invalid etag",
      })),
    ),
  );

  return z.array(MarketGroup).parse(names);
}

/**
 * Returns a map of id->name.
 */
export async function item_names(): Promise<Map<number, [string, string | undefined]>> {
  const market_groups = await market_group_names();
  const allIds = market_groups.map((group) => group.types).flat(1);

  // Check cache for existing items
  const cachedItems = await db.itemNames.bulkGet(allIds);
  const cachedMap = new Map<number, [string, string | undefined]>();

  for (const item of cachedItems) {
    if (item) {
      cachedMap.set(item.id, [item.name, item.category]);
    }
  }

  // Find missing IDs that need to be fetched
  const missingIds = allIds.filter(id => !cachedMap.has(id));

  // Batch missing IDs for API calls
  const ids = [];
  for (let i = 0; i < missingIds.length; i += 1000) {
    ids.push(missingIds.slice(i, i + 1000));
  }

  const name_promises = ids.map((idBatch) => esi.post("universe/names", { json: idBatch }));
  const name_results = await Promise.allSettled(name_promises);

  const itemsToStore: UniverseName[] = [];

  let requestFail: string = "";

  for (const result of name_results) {
    if (result.status === "fulfilled") {
      try {
        const names = z.array(UniverseName).parse(await result.value.json());
        for (const item of names) {
          cachedMap.set(item.id, [item.name, item.category]);
          itemsToStore.push({
            id: item.id,
            name: item.name,
            category: item.category
          });
        }
      } catch (error) {
        requestFail = `Failed to parse names response: ${error}`;
      }
    } else {
      requestFail = `Failed to fetch names: ${result.reason}`;
    }
  }

  // Store new items in IndexedDB
  if (itemsToStore.length > 0) {
    await db.itemNames.bulkAdd(itemsToStore);
  }

  if (requestFail !== "") {
    throw new Error("Request Failed");
  }

  return cachedMap;
}
