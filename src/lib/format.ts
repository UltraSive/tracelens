export const fmtMs = (v: number | null | undefined, digits = 1) =>
	v == null || Number.isNaN(v) ? "–" : v.toFixed(digits);

/** Tailwind text color for a loss percentage. */
export function lossClass(pct: number, sent: number) {
	if (sent === 0) return "text-muted-foreground";
	if (pct === 0) return "text-emerald-600 dark:text-emerald-400";
	if (pct < 5) return "text-amber-600 dark:text-amber-400";
	return "text-red-600 dark:text-red-400";
}
