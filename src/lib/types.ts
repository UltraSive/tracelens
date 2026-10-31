export type Protocol = "icmp" | "udp" | "tcp";
export type AddrFamily = "any" | "v4" | "v6";
export type TraceStatus = "running" | "finished" | "stopped" | "failed";

export interface TraceOptions {
	target: string;
	protocol: Protocol;
	family: AddrFamily;
	intervalMs: number;
	maxTtl: number;
	packetSize: number;
	maxRounds: number | null;
	port: number | null;
	resolveHostnames: boolean;
}

export interface HostEntry {
	ip: string;
	hostname: string | null;
	count: number;
}

export interface HopStats {
	ttl: number;
	hosts: HostEntry[];
	sent: number;
	recv: number;
	lossPct: number;
	lastMs: number | null;
	bestMs: number | null;
	worstMs: number | null;
	avgMs: number;
	stddevMs: number;
	jitterMs: number | null;
	/** oldest first, null = lost */
	samples: (number | null)[];
	isTarget: boolean;
}

export interface TraceSnapshot {
	id: number;
	target: string;
	targetAddr: string;
	protocol: Protocol;
	status: TraceStatus;
	rounds: number;
	hops: HopStats[];
	error: string | null;
}

export interface EnvInfo {
	privileges: { privileged: boolean; needsPrivileges: boolean };
	hint: string;
	os: string;
}
