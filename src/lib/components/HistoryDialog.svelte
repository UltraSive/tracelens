<script lang="ts">
	import * as Dialog from "$lib/components/ui/dialog";
	import { Button } from "$lib/components/ui/button";
	import { Checkbox } from "$lib/components/ui/checkbox";
	import TrashIcon from "@lucide/svelte/icons/trash-2";
	import { store } from "$lib/traces.svelte";
	import { fmtDateTime } from "$lib/format";
	import type { SavedMeta } from "$lib/types";
	import { toast } from "svelte-sonner";

	let { open = $bindable(false), onCompare }: { open: boolean; onCompare: (a: string, b: string) => void } = $props();

	let items = $state<SavedMeta[]>([]);
	let picked = $state<string[]>([]);
	let loading = $state(false);

	$effect(() => {
		if (open) refresh();
	});

	async function refresh() {
		loading = true;
		try {
			items = await store.listSaved();
		} catch (e) {
			toast.error(String(e));
		} finally {
			loading = false;
		}
	}

	function toggle(file: string, on: boolean) {
		picked = on ? [...picked.filter((f) => f !== file), file].slice(-2) : picked.filter((f) => f !== file);
	}

	async function del(m: SavedMeta) {
		await store.deleteSaved(m.file);
		picked = picked.filter((f) => f !== m.file);
		items = items.filter((i) => i.file !== m.file);
		toast(`Deleted trace to ${m.target}`);
	}

	const dot = { pending: "bg-zinc-400", ok: "bg-emerald-500", info: "bg-sky-500", warn: "bg-amber-500", bad: "bg-red-500" };
</script>

<Dialog.Root bind:open>
	<Dialog.Content class="sm:max-w-3xl">
		<Dialog.Header>
			<Dialog.Title>Saved traces</Dialog.Title>
			<Dialog.Description>
				Save a running trace with the Save button. Open one to review or export it, or tick two to compare them.
			</Dialog.Description>
		</Dialog.Header>
		<div class="max-h-[55vh] overflow-auto rounded-md border">
			{#if loading && !items.length}
				<div class="p-6 text-center text-sm text-muted-foreground">Loading…</div>
			{:else if !items.length}
				<div class="p-6 text-center text-sm text-muted-foreground">No saved traces yet.</div>
			{:else}
				{#each items as m (m.file)}
					<div class="flex items-center gap-3 border-b px-3 py-2 text-sm last:border-b-0 hover:bg-muted/40">
						<Checkbox checked={picked.includes(m.file)} onCheckedChange={(v) => toggle(m.file, v === true)} aria-label="Select for comparison" />
						<button class="min-w-0 flex-1 text-left" onclick={() => { store.openSaved(m); open = false; }}>
							<div class="flex items-center gap-2">
								<span class="size-1.5 shrink-0 rounded-full {dot[m.level]}"></span>
								<span class="font-mono">{m.target}</span>
								<span class="text-xs text-muted-foreground">{m.protocol.toUpperCase()} · {m.rounds} rounds · {m.hops} hops</span>
							</div>
							<div class="truncate pl-3.5 text-xs text-muted-foreground">{m.headline}</div>
						</button>
						<span class="shrink-0 text-xs text-muted-foreground">{fmtDateTime(m.savedAt)}</span>
						<Button variant="ghost" size="icon-xs" aria-label="Delete" onclick={() => del(m)}><TrashIcon /></Button>
					</div>
				{/each}
			{/if}
		</div>
		<Dialog.Footer>
			<span class="mr-auto self-center text-xs text-muted-foreground">{picked.length}/2 selected</span>
			<Button variant="outline" disabled={picked.length !== 2} onclick={() => { onCompare(`S${picked[0]}`, `S${picked[1]}`); open = false; }}>Compare selected</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>
