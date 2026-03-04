<script lang="ts">
import { pingBackend } from "$lib/tauriCommands";

let greeting = "Hello from Svelte";
let pingResponse = "";
let errorMessage = "";
let loading = false;

async function handlePing() {
	loading = true;
	errorMessage = "";
	try {
		pingResponse = await pingBackend("Hello world");
	} catch (error) {
		errorMessage = error instanceof Error ? error.message : String(error);
	} finally {
		loading = false;
	}
}
</script>

<main>
  <h1>{greeting}</h1>
  <button
    type="button"
    on:click={handlePing}
    disabled={loading}
  >
    Ping backend
  </button>
  {#if loading}
    <p>Loading...</p>
  {/if}
  {#if pingResponse}
    <p data-testid="ping-response">{pingResponse}</p>
  {/if}
  {#if errorMessage}
    <p class="error" data-testid="ping-error">{errorMessage}</p>
  {/if}
</main>

<style>
  main {
    font-family: "Arial", sans-serif;
    padding: 1rem;
  }

  .error {
    color: #b00020;
  }
</style>
