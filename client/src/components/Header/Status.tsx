import { useQuery } from "@tanstack/react-query";
import ky from "ky";
import React from "react";

export default function Status() {
  const esi_status = useQuery({
    queryKey: ["esi-status"],
    queryFn: async () => {
      return await ky.get("https://esi.evetech.net/status.json");
    },
  });

  const union_status = esi_status.status;

  return (
    <div className="relative group text-sm font-mono">
      <div className="flex items-center px-2 cursor-pointer">
        <Clock />
        <StatusDot status={union_status} />
      </div>

      <div className="absolute w-full top-full px-2 pt-1 hidden cursor-default group-hover:flex flex-col border-t border-white">
        <div className="flex items-center">
          ESI Status: <StatusDot status={esi_status.status} />{" "}
        </div>
      </div>
    </div>
  );
}

function Clock() {
  const timeOptions: [string, Intl.DateTimeFormatOptions] = ["en-US", { timeZone: "UTC", hour12: false }];
  const [time, setTime] = React.useState(new Date().toLocaleTimeString(...timeOptions));

  React.useEffect(() => {
    // Calculate delay to next second boundary
    const now = new Date();
    const msUntilNextSecond = 1000 - now.getMilliseconds();
    let intervalId: number;

    // Set initial timeout to align with second boundary
    const alignTimeout = setTimeout(() => {
      setTime(new Date().toLocaleTimeString(...timeOptions));

      // Now start the regular interval
      const interval = setInterval(() => {
        setTime(new Date().toLocaleTimeString(...timeOptions));
      }, 1000);

      // Store interval ID for cleanup
      intervalId = interval;
    }, msUntilNextSecond);

    return () => {
      clearTimeout(alignTimeout);
      if (intervalId) {
        clearInterval(intervalId);
      }
    };
  }, []);

  return <div className="font-mono">{time} EVE</div>;
}

function StatusDot({ status }: { status: "error" | "success" | "pending" }) {
  return (
    <div
      className={`rounded-full p-1 w-0 h-0 ${status === "error" ? "bg-red-500" : status === "pending" ? "bg-yellow-300" : "bg-green-400"} ml-2`}
    ></div>
  );
}
