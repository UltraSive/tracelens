import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { EnvInfo, ExportFormat, SavedMeta, TraceOptions, TraceSnapshot } from "./types";

export interface Tab {
	key: string;
	kind: "live" | "saved";
	snap: TraceSnapshot;
	/** saved-trace file name */
	file?: string;
	savedAt?: number;
}

const liveKey = (id: number) => `L${id}`;

class TraceStore {
	tabs = $state<Tab[]>([]);
	activeKey = $state<string | null>(null);
	env = $state<EnvInfo | null>(null);
	error = $state<string | null>(null);
	starting = $state(false);
	/** hop selected in the table (drives the chart) */
	selectedTtl = $state<number | null>(null);

	active = $derived(this.tabs.find((t) => t.key === this.activeKey) ?? null);

	async init() {
		this.env = await invoke<EnvInfo>("env_info");
		this.merge(await invoke<TraceSnapshot[]>("snapshots"));
		// a trace started from the command line should be shown immediately
		const first = this.tabs.at(-1);
		if (first) this.select(first.key);
		return listen<TraceSnapshot[]>("traces", (e) => this.merge(e.payload));
	}

	/** Merge live snapshots into the tab list, keeping saved tabs and tab order. */
	private merge(snaps: TraceSnapshot[]) {
		const byId = new Map(snaps.map((s) => [s.id, s]));
		const next: Tab[] = [];
		for (const t of this.tabs) {
			if (t.kind === "saved") next.push(t);
			else {
				const s = byId.get(t.snap.id);
				if (s) {
					next.push({ ...t, snap: s });
					byId.delete(t.snap.id);
				}
			}
		}
		for (const s of byId.values()) next.push({ key: liveKey(s.id), kind: "live", snap: s });
		this.tabs = next;
		if (this.activeKey && !next.some((t) => t.key === this.activeKey)) this.select(next.at(-1)?.key ?? null);
	}

	select(key: string | null) {
		if (key !== this.activeKey) this.selectedTtl = null;
		this.activeKey = key;
		const tab = this.tabs.find((t) => t.key === key);
		invoke("set_active", { id: tab?.kind === "live" ? tab.snap.id : null });
	}

	selectIndex(i: number) {
		const t = this.tabs[i];
		if (t) this.select(t.key);
	}

	cycle(dir: 1 | -1) {
		if (!this.tabs.length) return;
		const i = this.tabs.findIndex((t) => t.key === this.activeKey);
		this.selectIndex((i + dir + this.tabs.length) % this.tabs.length);
	}

	async start(opts: TraceOptions) {
		this.error = null;
		this.starting = true;
		try {
			const snap = await invoke<TraceSnapshot>("start_trace", { opts });
			const key = liveKey(snap.id);
			if (!this.tabs.some((t) => t.key === key)) this.tabs = [...this.tabs, { key, kind: "live", snap }];
			this.select(key);
			return true;
		} catch (e) {
			this.error = String(e);
			return false;
		} finally {
			this.starting = false;
		}
	}

	async togglePause(tab: Tab) {
		if (tab.kind !== "live") return;
		const s = tab.snap.status;
		if (s !== "running" && s !== "paused") return;
		await invoke("set_paused", { id: tab.snap.id, paused: s === "running" });
		tab.snap.status = s === "running" ? "paused" : "running"; // optimistic
	}

	async restart(tab: Tab) {
		if (tab.kind === "saved") {
			await this.start(tab.snap.options);
			return;
		}
		try {
			const snap = await invoke<TraceSnapshot>("restart_trace", { id: tab.snap.id });
			const key = liveKey(snap.id);
			this.tabs = this.tabs.map((t) => (t.key === tab.key ? { key, kind: "live", snap } : t));
			if (this.activeKey === tab.key) this.select(key);
		} catch (e) {
			this.error = String(e);
		}
	}

	async reset(tab: Tab) {
		if (tab.kind === "live") await invoke("reset_trace", { id: tab.snap.id });
	}

	async close(tab: Tab) {
		if (tab.kind === "live") await invoke("remove_trace", { id: tab.snap.id });
		const idx = this.tabs.findIndex((t) => t.key === tab.key);
		this.tabs = this.tabs.filter((t) => t.key !== tab.key);
		if (this.activeKey === tab.key) this.select(this.tabs[Math.min(idx, this.tabs.length - 1)]?.key ?? null);
	}

	/** Full-resolution snapshot (live traces otherwise only stream what's on screen). */
	async fullSnapshot(tab: Tab): Promise<TraceSnapshot> {
		if (tab.kind === "saved") return tab.snap;
		return invoke<TraceSnapshot>("snapshot", { id: tab.snap.id });
	}

	async render(tab: Tab, format: ExportFormat) {
		return invoke<string>("render_export", { snapshot: await this.fullSnapshot(tab), format });
	}

	async save(tab: Tab) {
		if (tab.kind !== "live") return null;
		return invoke<SavedMeta>("save_trace", { id: tab.snap.id });
	}

	listSaved() {
		return invoke<SavedMeta[]>("list_saved");
	}

	loadSaved(file: string) {
		return invoke<TraceSnapshot>("load_saved", { file });
	}

	deleteSaved(file: string) {
		this.tabs = this.tabs.filter((t) => t.file !== file);
		return invoke("delete_saved", { file });
	}

	async openSaved(meta: SavedMeta) {
		const key = `S${meta.file}`;
		if (!this.tabs.some((t) => t.key === key)) {
			const snap = await this.loadSaved(meta.file);
			this.tabs = [...this.tabs, { key, kind: "saved", snap, file: meta.file, savedAt: meta.savedAt }];
		}
		this.select(key);
	}
}

export const store = new TraceStore();
