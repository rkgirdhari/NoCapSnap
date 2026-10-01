<script lang="ts">
  import { onMount } from "svelte";
  import { bridge, type CaptureRecord, type Profile } from "$lib/bridge";
  import Icon from "$lib/components/Icon.svelte";
  import PlateRow from "$lib/components/PlateRow.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import { dayLabel, whenLabel } from "$lib/format";

  let captures = $state<CaptureRecord[]>([]);
  let profile = $state<Profile | null>(null);
  let loaded = $state(false);
  let error = $state<string | null>(null);
  let syncing = $state(false);
  let syncNote = $state<string | null>(null);

  async function load() {
    try {
      [captures, profile] = await Promise.all([bridge.list(500), bridge.profile()]);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loaded = true;
    }
  }

  onMount(() => {
    load();
    let off: (() => void) | undefined;
    bridge.onSyncUpdated(load).then((u) => (off = u));
    return () => off?.();
  });

  async function retry() {
    syncing = true;
    syncNote = null;
    try {
      const r = await bridge.syncNow();
      syncNote = r.stopped
        ? `Not synced: ${r.stopped}.`
        : r.refused
          ? `${r.synced} synced; ${r.refused} refused by the server (see the plate).`
          : `${r.synced} synced.`;
    } catch (e) {
      syncNote = e instanceof Error ? e.message : String(e);
    } finally {
      syncing = false;
      await load();
    }
  }

  // Demo plates never sync, so they don't count as waiting.
  const pending = $derived(captures.filter((c) => c.syncState === "pending" && !c.isDemo).length);
  // The newest server acknowledgement on this device, if any ever happened.
  const lastSynced = $derived(
    captures.reduce<string | null>((max, c) => (c.syncedAt && (!max || c.syncedAt > max) ? c.syncedAt : max), null),
  );
  const groups = $derived.by(() => {
    const out: { day: string; items: CaptureRecord[] }[] = [];
    for (const c of captures) {
      const day = dayLabel(c.capturedAt);
      if (out.at(-1)?.day !== day) out.push({ day, items: [] });
      out.at(-1)!.items.push(c);
    }
    return out;
  });
</script>

<TopBar status />

<main class="page">
  <h1 class="display center">Your plates are safe here.</h1>

  {#if pending > 0}
    <div class="card banner" role="status">
      <Icon name="database" size={40} stroke={1.3} />
      <span class="rule" aria-hidden="true"></span>
      <span>
        <strong>{pending} saved offline</strong>
        <span class="muted">Saved on this device. Guest QR not ready.</span>
      </span>
    </div>
  {/if}
  {#if loaded && captures.length}
    <p class="last"><span>Last synced</span> <span class="muted">{lastSynced ? whenLabel(lastSynced) : "Never"}</span></p>
  {/if}

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if loaded && !captures.length && !error}
    <div class="empty">
      <p class="muted">No plates yet. Captures are saved on this device first, then sync.</p>
      <a class="btn primary block tall" href="/capture"><Icon name="camera" size={26} /> Capture a dish</a>
    </div>
  {/if}

  {#each groups as group (group.day)}
    <h2 class="title day">{group.day}</h2>
    <div class="list">
      {#each group.items as capture (capture.clientId)}
        <PlateRow {capture} />
      {/each}
    </div>
  {/each}

  {#if pending > 0}
    {#if profile?.signedIn}
      <button class="btn gold block tall retry" onclick={retry} disabled={syncing}>
        <Icon name="sync" size={26} stroke={1.6} />
        {syncing ? "Syncing…" : "Retry when connected"}
      </button>
      {#if !syncNote}<p class="helper center">It also retries by itself every minute while the app is open.</p>{/if}
    {:else}
      <a class="btn gold block tall retry" href="/settings"><Icon name="sync" size={26} stroke={1.6} /> Sign in to sync</a>
      <p class="helper center">Plates wait safely on this phone until you sign in.</p>
    {/if}
  {/if}
  <!-- Outside the block above: after a full sync nothing is waiting, but the result still shows. -->
  {#if syncNote}<p class="helper center" role="status">{syncNote}</p>{/if}
</main>

<style>
  .center {
    text-align: center;
  }
  .display {
    margin-top: 12px;
    font-size: clamp(36px, 11vw, 48px);
  }
  .banner {
    display: grid;
    grid-template-columns: auto auto 1fr;
    align-items: center;
    gap: 16px;
    margin-top: 22px;
    padding: 16px 18px;
    color: var(--text);
  }
  .banner strong {
    display: block;
    font-size: 19px;
    font-weight: 550;
  }
  .banner .muted {
    font-size: 15px;
  }
  .rule {
    width: 1px;
    align-self: stretch;
    background: var(--hairline);
  }
  .last {
    display: flex;
    gap: 14px;
    margin: 12px 4px 0;
    font-size: 15px;
    letter-spacing: 0.03em;
  }
  .day {
    margin: 26px 0 12px;
    font-size: 28px;
  }
  .list {
    display: grid;
    gap: 10px;
  }
  .empty {
    display: grid;
    gap: 16px;
    margin-top: 24px;
    text-align: center;
  }
  .retry {
    margin-top: 22px;
  }
  .retry:disabled {
    opacity: 0.7;
  }
  .helper {
    margin: 10px 0 0;
  }
</style>
