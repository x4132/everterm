import InvalidID from "@/components/InvalidID";
import { db, UniverseType } from "@/db";
import esi from "@/lib/esiClient";
import { MarketOrder, MarketOrderBook, RefreshIntervals, Region, Station } from "@/lib/schemas";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { createFileRoute } from "@tanstack/react-router";
import ky from "ky";
import React, { useState } from "react";

export const Route = createFileRoute("/market/$itemId/mdm")({
  loader: async ({ params }) => {
    return parseInt(params.itemId);
  },
  component: RouteComponent,
});

let updateTimeout: number | null = null;

function RouteComponent() {
  const itemId = Route.useLoaderData();
  const queryClient = useQueryClient();

  if (isNaN(itemId)) {
    return <InvalidID />;
  }

  const orders = useQuery({
    queryKey: ["orderbook", itemId],
    queryFn: async () => {
      return MarketOrderBook.parse(await ky.get(`/api/orders/${itemId}`).json()).sort((a, b) => a.price - b.price);
    },
  });

  const item = useQuery({
    queryKey: ["item", itemId],
    queryFn: async () => {
      return UniverseType.parse(await esi.get(`universe/types/${itemId}/`).json());
    },
  });

  const refreshTime = useQuery({
    queryKey: ["refreshTime"],
    queryFn: async () => {
      return RefreshIntervals.parse(await ky.get("/api/orders/updateTime").json());
    },
  });

  const nextRefresh = refreshTime.data
    ? Object.entries(refreshTime.data).sort((a, b) => a[1].getTime() - b[1].getTime())
    : [];

  if (!updateTimeout && nextRefresh[0] && nextRefresh[0][1].getTime() < Date.now()) {
    updateTimeout = setTimeout(() => {
      updateTimeout = null;
      queryClient.invalidateQueries({ queryKey: ["orderbook"] });
      queryClient.invalidateQueries({ queryKey: ["refreshTime"] });
    }, 10000);
  }

  return (
    <div>
      {item.isPending ? <div>Loading Item Info...</div> : null}
      {item.isError ? <div>Error: {item.error.message}</div> : null}
      {item.isSuccess ? (
        <div className="flex p-2 items-center">
          {/* <img
            src={`https://images.evetech.net/types/${itemId}/icon`}
            alt={"Icon"}
            className="border rounded mr-2 h-16 w-16"
          /> */}
          <h1 className="h1">{item.data.name}</h1>

          <CountdownClock updateTime={nextRefresh} />
        </div>
      ) : null}

      {orders.isPending ? <div>Loading Orders...</div> : null}
      {orders.isError ? <div>Error: {orders.error.message}</div> : null}
      {orders.isSuccess ? (
        <div className="m-2">
          <div className="overflow-auto max-h-[40vh] border-y border-gray-300">
            <table className="min-w-full border border-t-0 border-gray-300 border-collapse">
              <thead className="top-0 border border-gray-300 border-t-0 sticky bg-background text-left">
                <tr className="w-full">
                  <th className="px-2 text-xl font-bold" colSpan={4}>
                    Sellers
                  </th>
                </tr>
                <tr className="">
                  <th className="px-2">Qty</th>
                  <th className="px-2">Price</th>
                  <th className="px-2">Location</th>
                  <th className="px-2">Expires</th>
                </tr>
              </thead>
              <tbody className="overflow-auto max-h-9">
                {orders.data
                  .filter((order) => !order.is_buy_order)
                  .map((order) => (
                    <OrderRow order={order} key={order.id} />
                  ))}
              </tbody>
            </table>
          </div>
          <div className="overflow-auto max-h-[40vh] border-y border-gray-300 mt-6">
            <table className="min-w-full border border-t-0 border-gray-300 border-collapse">
              <thead className="top-0 border border-gray-300 border-t-0 sticky bg-background text-left">
                <tr className="w-full">
                  <th className="px-2 text-xl font-bold" colSpan={4}>
                    Buyers
                  </th>
                </tr>
                <tr className="">
                  <th className="px-2">Qty</th>
                  <th className="px-2">Price</th>
                  <th className="px-2">Location</th>
                  <th className="px-2">Expires</th>
                </tr>
              </thead>
              <tbody className="overflow-auto max-h-9">
                {orders.data
                  .filter((order) => order.is_buy_order)
                  .reverse()
                  .map((order) => (
                    <OrderRow order={order} key={order.id} />
                  ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : null}
    </div>
  );
}

function OrderRow({ order }: { order: MarketOrder }) {
  const info = useQuery({
    queryKey: ["structureInfo", order.location_id],
    queryFn: async () => {
      let url = new URL("/api/universe/struct_names/", location.origin);
      url.searchParams.append("id", order.location_id + "");

      return Station.parse(await ky.get(url).json());
    },
  });

  return (
    <tr key={order.id}>
      <td className="px-2 border border-gray-300">{order.volume_remain}</td>
      <td className="px-2 border border-gray-300">{order.price.toLocaleString("en-US")}</td>
      <td className="px-2 border border-gray-300">
        {info.status === "pending" ? "Loading..." : ""}
        {info.status === "error" ? "Error" : ""}
        {info.status === "success" ? info.data.name : ""}
      </td>
      <td className="px-2 border border-gray-300">
        {(() => {
          const diffMs = new Date(order.expiry).getTime() - Date.now();
          if (diffMs <= 0) return "Expired";
          let remaining = diffMs;
          const parts: string[] = [];
          for (const [label, ms] of [
            ["d", 86400000],
            ["h", 3600000],
            ["m", 60000],
          ] as const) {
            const value = Math.floor(remaining / ms);
            if (value > 0) {
              parts.push(`${value}${label}`);
              remaining %= ms;
            }
          }
          return parts.length > 0 ? parts.join(" ") : "< 1m";
        })()}
      </td>
    </tr>
  );
}

function CountdownClock({ updateTime }: { updateTime: [string, Date][] }) {
  const [regionTime, setTime] = useState<[string, number]>(["0", 0]);
  const time = Math.floor(regionTime[1] / 1000);

  const region = useQuery({
    queryKey: ["region", regionTime[0]],
    queryFn: async () => {
      let cached = await db.regionNames.get(parseInt(regionTime[0]))
      if (cached) return cached.name;

      let region = Region.parse(await esi.get(`universe/regions/${regionTime[0]}`).json());
      await db.regionNames.add(region);

      return region.name;
    },
  });

  React.useEffect(() => {
    const now = new Date();
    const msUntil = 1000 - now.getMilliseconds();
    let intervalId: number;

    const alignTimeout = setTimeout(() => {
      let deltas = updateTime.map((cur) => [cur[0], cur[1].getTime() - Date.now()] as [string, number]);
      setTime(deltas.reduce((prev, cur) => (cur[1] >= 0 && prev[1] < 0 ? cur : prev), ["0", -1]));

      intervalId = setInterval(() => {
        let deltas = updateTime.map((cur) => [cur[0], cur[1].getTime() - Date.now()] as [string, number]);
        setTime(deltas.reduce((prev, cur) => (cur[1] >= 0 && prev[1] < 0 ? cur : prev), ["0", -1]));
      }, 1000);
    }, msUntil);

    return () => {
      clearTimeout(alignTimeout);
      if (intervalId) clearInterval(intervalId);
    };
  }, [updateTime]);

  return (
    <div className="flex flex-col text-center ml-auto px-2">
      <span className="text-sm">{region.data ? region.data : region.status} Refreshes In:</span>
      <span className="h1">
        {time !== 0 ? `${Math.floor(time / 60)}:${((time % 60) + "").padStart(2, "0")}` : "...."}
      </span>
    </div>
  );
}
