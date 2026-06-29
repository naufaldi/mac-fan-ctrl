import "@testing-library/jest-dom/vitest";
import { cleanup } from "@testing-library/svelte";
import { afterEach } from "vitest";

afterEach(() => {
	cleanup();
});

if (!Element.prototype.animate) {
	Element.prototype.animate = function animate() {
		return {
			cancel: () => {},
			finish: () => {},
			pause: () => {},
			play: () => {},
			reverse: () => {},
			addEventListener: () => {},
			removeEventListener: () => {},
			finished: Promise.resolve(this),
		} as unknown as Animation;
	};
}
