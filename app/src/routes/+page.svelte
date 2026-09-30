<script lang="ts">
  import { onMount } from "svelte";
  import { bridge, type CaptureRecord, type Profile, type StoreStatus } from "$lib/bridge";
  import Icon from "$lib/components/Icon.svelte";
  import PlateRow from "$lib/components/PlateRow.svelte";
  import Thumb from "$lib/components/Thumb.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import { greeting, isSameLocalDay, serviceLine } from "$lib/format";

  let profile = $state<Profile | null>(null);
  let status = $state<StoreStatus | null>(null);
  let captures = $state<CaptureRecord[]>([]);
  let error = $state<string | null>(null);

  const now = new Date();
  const today = $derived(captures.filter((c) => isSameLocalDay(new Date(c.capturedAt), now)));
  const hero = $derived(today[0] ?? null);
  const serviceWord = now.getHours() >= 16 || now.getHours() < 5 ? "Tonight's service" : "Today's service";
  const firstName = $derived(profile?.displayName?.split(/\s+/)[0] ?? null);

  async function load() {
    try {
      [profile, status, captures] = await Promise.all([bridge.profile(), bridge.status(), bridge.list(200)]);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  onMount(() => {
    load();
    let off: (() => void) | undefined;
    bridge.onSyncUpdated(load).then((u) => (off = u));
    return () => off?.();
  });
</script>

<TopBar status />

<main class="page">
  <p class="kicker">{serviceLine(now)}</p>
  <h1 class="display greet">{greeting(now)}{firstName ? `, ${firstName}` : ""}.</h1>

  {#if profile?.locationName}
    <p class="location caps">
      {profile.locationName}{#if profile.isDemo}<span class="demo">Demo</span>{/if}
    </p>
  {/if}

  <section class="hero" aria-label={serviceWord}>
    {#if hero}
      <Thumb sha256={hero.sha256} thumb={false} radius="var(--radius-lg)" alt={hero.dishName ?? "Latest plate"} />
    {:else}
      <div class="hero-empty" aria-hidden="true">
        <svg viewBox="0 0 100 100"><circle cx="50" cy="50" r="40" /><circle cx="50" cy="50" r="28" /></svg>
      </div>
    {/if}
    <div class="hero-text">
      <span>{serviceWord}</span>
      <strong
        >{#if today.length === 0}No captures yet{:else}{today.length}
          {today.length === 1 ? "capture" : "captures"}{/if}</strong
      >
    </div>
  </section>

  <a class="btn primary block tall cta" href="/capture">
    <Icon name="camera" size={26} stroke={1.7} /> Capture a dish
  </a>

  <div class="stats">
    <a class="stat" href="/history">
      <Icon name="stack" size={22} stroke={1.4} />
      <span><strong>{status?.pending ?? "–"}</strong>saved offline</span>
      <Icon name="chevron-right" size={16} stroke={1.6} />
    </a>
    <a class="stat" href="/history">
      <Icon name="qr" size={22} stroke={1.4} />
      <span><strong>{status?.synced ?? "–"}</strong>QR ready</span>
      <Icon name="chevron-right" size={16} stroke={1.6} />
    </a>
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <div class="recent-head">
    <h2 class="title">Recent plates</h2>
    {#if captures.length}
      <a class="caps see-all" href="/history">See all <Icon name="chevron-right" size={18} /></a>
    {/if}
  </div>

  {#if captures.length}
    <div class="recent">
      {#each captures.slice(0, 3) as capture (capture.clientId)}
        <PlateRow {capture} withDay />
      {/each}
    </div>
  {:else}
    <p class="helper">Plates you capture appear here, saved on this device first.</p>
  {/if}
</main>

<style>
  .greet {
    margin-top: 8px;
  }

  .location {
    display: inline-flex;
    align-items: center;
    gap: 10px;
    margin: 14px 0 0;
    padding: 7px 12px;
    border-radius: 8px;
    background: var(--stat);
    border: 1px solid var(--edge);
    color: var(--text);
  }
  .demo {
    padding: 2px 7px;
    border-radius: 5px;
    background: var(--wait-bg);
    color: var(--wait);
    font-size: 11px;
    letter-spacing: 0.14em;
  }

  .hero {
    position: relative;
    margin-top: 16px;
    aspect-ratio: 1.58;
    border-radius: var(--radius-lg);
    overflow: hidden;
    border: 1px solid var(--hairline);
  }
  .hero-empty {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: radial-gradient(circle at 50% 40%, #33222f, #1b1020 70%);
  }
  .hero-empty svg {
    width: 44%;
    fill: none;
    stroke: var(--muted);
    stroke-width: 0.8;
    opacity: 0.6;
  }
  .hero-text {
    position: absolute;
    inset: auto 0 0 0;
    display: grid;
    gap: 2px;
    padding: 40px 18px 16px;
    background: linear-gradient(transparent, rgba(16, 7, 12, 0.82));
    color: var(--headline);
  }
  .hero-text span {
    font-size: 15px;
    letter-spacing: 0.02em;
  }
  .hero-text strong {
    font-family: var(--serif);
    font-size: 25px;
    font-weight: 700;
    line-height: 1.1;
  }

  .cta {
    margin-top: 12px;
    font-size: 17.5px;
  }

  .stats {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 10px;
    margin-top: 10px;
  }
  .stat {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: center;
    gap: 8px;
    min-height: 64px;
    padding: 8px 8px 8px 10px;
    border-radius: var(--radius);
    background: var(--stat);
    border: 1px solid var(--edge);
    color: var(--stat-label);
    text-decoration: none;
    font-size: 13px;
    line-height: 1.2;
    white-space: nowrap;
  }
  .stat strong {
    display: block;
    font-size: 19px;
    font-weight: 600;
    color: var(--text);
  }

  .recent-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin: 24px 0 10px;
  }
  .see-all {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--text);
    text-decoration: none;
  }
  .recent {
    display: grid;
    gap: 10px;
  }
</style>
