import { mount } from "svelte";
import App from "./App.svelte";

const appRoot = document.getElementById("app");

if (!appRoot) {
  throw new Error("App root element not found");
}

const app = mount(App, {
  target: appRoot,
});

export default app;
