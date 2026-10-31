<script lang="ts">
	import "../app.css";
	import { dev } from "$app/environment";
	import * as Tooltip from "$lib/components/ui/tooltip";
	import { Toaster } from "$lib/components/ui/sonner";

	let { children } = $props();

	// The webview's "Reload / Inspect" context menu is a dead giveaway; keep it only in dev
	// and on text fields where copy/paste is useful. Our own context menus still work.
	function onContextMenu(e: MouseEvent) {
		if (dev || e.defaultPrevented) return;
		const el = e.target as HTMLElement;
		if (!el.closest("input, textarea, .selectable")) e.preventDefault();
	}
</script>

<svelte:window oncontextmenu={onContextMenu} />

<Tooltip.Provider delayDuration={300}>
	{@render children()}
</Tooltip.Provider>
<Toaster position="bottom-right" richColors={false} closeButton />
