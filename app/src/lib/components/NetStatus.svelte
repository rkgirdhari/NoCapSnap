<script lang="ts">
  import { bridge } from "$lib/bridge";
  import { connectivity } from "$lib/connectivity.svelte";
  import Icon from "./Icon.svelte";

  // Network reachability, plus a sync-now button that says what happened.
  let syncing = $state(false);
  let note = $state<string | null>(null);
  let clear: ReturnType<typeof setTimeout> | undefined;

  function show(text: string) {
    note = text;
    clearTimeout(clear);
    clear = setTimeout(() => (note = null), 4000);
  }

  async function syncNow() {
    if (syncing) return;
    syncing = true;
    try {
      const r = await bridge.syncNow();
      show(r.stopped ? `Not synced: ${r.stopped}` : r.synced ? `${r.synced} synced` : "Nothing to sync");
    } catch (e) {
      const signedIn = await bridge.profile().then((p) => p.signedIn, () => false);
      show(signedIn ? (e instanceof Error ? e.message : String(e)) : "Sign in to sync (Settings)");
    } finally {
      syncing = false;
    }
  }
</script>

<span class="net" class:offline={!connectivity.online}>
  {#if connectivity.online}
    <button class="sync" class:spinning={syncing} onclick={syncNow} disabled={syncing} aria-label="Sync now">
      <Icon name="sync" size={21} />
    </button>
  {:else}
    <Icon name="cloud-off" size={21} />
  {/if}
  <span class="dot" aria-hidden="true"></span>
  <span role="status">{note ?? (connectivity.online ? "Online" : "Offline")}</span>
</span>

<style>
  .net {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text);
  }
  .net :global(svg) {
    margin-right: 8px;
    color: var(--text);
  }
  .sync {
    display: inline-flex;
    margin: -10px 0 -10px -10px;
    padding: 10px 2px 10px 10px;
    border: 0;
    background: none;
    color: inherit;
    cursor: pointer;
  }
  .sync:disabled {
    cursor: default;
  }
  .spinning :global(svg) {
    animation: spin 0.9s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .spinning :global(svg) {
      animation: none;
      opacity: 0.5;
    }
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok-dot);
  }
  .offline {
    color: var(--wait-hi);
  }
  .offline :global(svg) {
    color: var(--wait-hi);
  }
  .offline .dot {
    background: var(--wait-hi);
  }
</style>
