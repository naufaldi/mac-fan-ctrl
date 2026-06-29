export type Distribution = "direct" | "app-store";

export function getDistribution(): Distribution {
	return import.meta.env.VITE_FANGUARD_DISTRIBUTION === "app-store"
		? "app-store"
		: "direct";
}

export function isAppStoreDistribution(): boolean {
	return getDistribution() === "app-store";
}
