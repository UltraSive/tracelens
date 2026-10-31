import type { TraceStatus } from "./types";

export const statusDot: Record<TraceStatus, string> = {
	running: "bg-emerald-500 animate-pulse",
	paused: "bg-amber-500",
	finished: "bg-sky-500",
	stopped: "bg-zinc-400",
	failed: "bg-red-500"
};
