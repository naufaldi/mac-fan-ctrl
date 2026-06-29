export const getCurrentWindow = (): {
	setAlwaysOnTop: (value: boolean) => Promise<void>;
} => ({
	setAlwaysOnTop: async () => {},
});
