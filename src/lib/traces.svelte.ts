import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { EnvInfo, TraceOptions, TraceSnapshot } from "./types";

class TraceStore {
	traces = $state<TraceSnapshot[]>([]);
	activeId = $state<number | null>(null);
	env = $state<EnvInfo | null>(null);
	error = $state<string | null>(null);
	starting = $state(false);

	active = $derived(this.traces.find((t) => t.id === this.activeId) ?? null);

	async init() {
		this.env = await invoke<EnvInfo>("env_info");
		this.traces = await invoke<TraceSnapshot[]>("snapshots");
		this.activeId = this.traces.at(-1)?.id ?? null;
		return listen<TraceSnapshot[]>("traces", (e) => {
			this.traces = e.payload;
		});
	}

	async start(opts: TraceOptions) {
		this.error = null;
		this.starting = true;
		try {
			const snap = await invoke<TraceSnapshot>("start_trace", { opts });
			this.traces = [...this.traces, snap];
			this.activeId = snap.id;
		} catch (e) {
			this.error = String(e);
		} finally {
			this.starting = false;
		}
	}

	async stop(id: number) {
		await invoke("stop_trace", { id });
	}

	async restart(id: number) {
		try {
			const snap = await invoke<TraceSnapshot>("restart_trace", { id });
			this.traces = this.traces.map((t) => (t.id === id ? snap : t));
			if (this.activeId === id) this.activeId = snap.id;
		} catch (e) {
			this.error = String(e);
		}
	}

	async reset(id: number) {
		await invoke("reset_trace", { id });
	}

	async remove(id: number) {
		await invoke("remove_trace", { id });
		const idx = this.traces.findIndex((t) => t.id === id);
		this.traces = this.traces.filter((t) => t.id !== id);
		if (this.activeId === id) {
			this.activeId = this.traces[Math.min(idx, this.traces.length - 1)]?.id ?? null;
		}
	}

	async exportText(id: number, format: "text" | "csv" | "json") {
		return invoke<string>("export_trace", { id, format });
	}
}

export const store = new TraceStore();
