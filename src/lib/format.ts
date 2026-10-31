import type { HopStats, HostEntry } from "./types";

export const fmtMs = (v: number | null | undefined, digits = 1) =>
	v == null || Number.isNaN(v) ? "–" : v.toFixed(digits);

/** Tailwind text color for a loss percentage. */
export function lossClass(pct: number, sent: number) {
	if (sent === 0) return "text-muted-foreground";
	if (pct === 0) return "text-emerald-600 dark:text-emerald-400";
	if (pct < 5) return "text-amber-600 dark:text-amber-400";
	return "text-red-600 dark:text-red-400";
}

/** "Your router", "Private network", "CLOUDFLARENET", or null while unknown. */
export function ownerName(hop: HopStats, host: HostEntry | undefined = hop.hosts[0]) {
	if (!host) return null;
	if (host.private) return hop.ttl === 1 ? "Your router" : "Private network";
	return host.asn?.name ?? null;
}

export const fmtTime = (ms: number) =>
	new Date(ms).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit", second: "2-digit" });

export const fmtDateTime = (ms: number) =>
	new Date(ms).toLocaleString(undefined, { dateStyle: "medium", timeStyle: "short" });

export const isMac = typeof navigator !== "undefined" && /Mac/.test(navigator.platform);
export const modKey = isMac ? "⌘" : "Ctrl";
