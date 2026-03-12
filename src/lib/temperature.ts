type TemperatureStatus = "unknown" | "normal" | "warm" | "hot";

const NORMAL_THRESHOLD = 70;
const WARM_THRESHOLD = 85;

export const getTemperatureStatus = (
	temp: number | null,
): TemperatureStatus => {
	if (temp === null) return "unknown";
	if (temp < NORMAL_THRESHOLD) return "normal";
	if (temp < WARM_THRESHOLD) return "warm";
	return "hot";
};
