<script lang="ts">
  import { onMount } from "svelte";
  import { page } from "$app/state";
  import { bridge, type CaptureRecord, type Profile } from "$lib/bridge";
  import Chip from "$lib/components/Chip.svelte";
  import GuestQr from "$lib/components/GuestQr.svelte";
  import Icon from "$lib/components/Icon.svelte";
  import Thumb from "$lib/components/Thumb.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import { whenLabel } from "$lib/format";

  // One capture's guest invitation. The QR exists only after the server has
  // acknowledged the capture (Spec §3), and it encodes the link the server
  // issued: `<server>/g/#<token>` (Spec §4).
  const clientId = page.url.searchParams.get("c");
  const fromCapture = page.url.searchParams.get("from") === "capture";

  let capture = $state<CaptureRecord | null>(null);
  let profile = $state<Profile | null>(null);
  let loaded = $state(false);
  let error = $state<string | null>(null);

  async function load() {
    try {
      [capture, profile] = await Promise.all([
        clientId ? bridge.get(clientId) : Promise.resolve(null),
        bridge.profile(),
      ]);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loaded = true;
    }
  }

  onMount(() => {
    load();
    // A plate that syncs while this screen is open turns into its QR.
    let off: (() => void) | undefined;
    bridge.onSyncUpdated(load).then((u) => (off = u));
    return () => off?.();
  });

  const expiry = new Intl.DateTimeFormat(undefined, { month: "long", day: "numeric" });
  const synced = $derived(capture?.syncState === "synced");
  const expired = $derived(!!capture?.guestExpiresAt && new Date(capture.guestExpiresAt) < new Date());
  const showQr = $derived(synced && !!capture?.guestUrl && !expired);
  const where = $derived(
    [
      capture?.isDemo ? "Demo" : profile && capture?.locationId === profile.locationId ? profile.locationName : null,
      capture?.tableLabel ? `Table ${capture.tableLabel}` : null,
    ].filter((p): p is string => !!p),
  );
</script>

<TopBar center label="Guest invitation" back={fromCapture ? undefined : "/history"} />

<main class="page invite">
  {#if fromCapture}
    <p class="kicker center">03 / 03 <span class="sep">·</span> Share</p>
  {/if}

  {#if !loaded}
    <p class="helper center">Loading…</p>
  {:else if !capture}
    <h1 class="display center">Plate not found.</h1>
    <p class="muted center">{error ?? "It isn't on this device."}</p>
    <a class="btn primary block tall" href="/history">Back to history</a>
  {:else}
    <h1 class="display center">{showQr ? "Ready to share." : synced ? "Synced." : "Saved on this device."}</h1>

    <div class="state-line">
      {#if showQr}
        <p class="state synced">
          <span class="badge" aria-hidden="true"><Icon name="check" size={22} stroke={2.2} /></span> Synced · QR ready
        </p>
      {:else if synced}
        <p class="state plain">Synced with your restaurant</p>
      {:else}
        <Chip state={capture.syncState} demo={capture.isDemo} />
      {/if}
    </div>

    <div class="card dish">
      <span class="pic"><Thumb sha256={capture.sha256} alt="" /></span>
      <span>
        <span class="dish-name">{capture.dishName ?? "Unlabelled dish"}</span>
        <span class="caps where">
          {#if where.length}
            {#each where as part, i (part)}{#if i}<span class="dot">·</span>{/if}<span class="nowrap">{part}</span>{/each}
          {:else}{whenLabel(capture.capturedAt)}{/if}
        </span>
      </span>
    </div>

    {#if showQr && capture.guestUrl}
      <div class="qr"><GuestQr url={capture.guestUrl} /></div>
      {#if capture.guestExpiresAt}
        <p class="concept">One use · expires {expiry.format(new Date(capture.guestExpiresAt))}</p>
      {/if}
      <p class="ask">Ask your guest to scan for private feedback.</p>
    {:else if synced}
      <p class="ask small">
        {expired && capture.guestExpiresAt
          ? `This plate's guest link expired on ${expiry.format(new Date(capture.guestExpiresAt))}.`
          : "This plate's guest link isn't on this phone; its feedback may already be in."}
      </p>
    {:else if capture.isDemo}
      <div class="qr-wait">
        <Icon name="plate" size={34} stroke={1.3} />
        <p>Demo plates stay on this phone and never get a guest QR.</p>
      </div>
      <p class="ask small">Sign in to your restaurant in Settings to share plates with guests.</p>
    {:else}
      <div class="qr-wait">
        <Icon name="hourglass" size={34} stroke={1.3} />
        <p>The guest QR appears here once this plate syncs.</p>
      </div>
      <p class="ask small">
        {#if !profile?.signedIn}
          Sign in to your restaurant in Settings to sync.
        {:else if capture.lastSyncError}
          Last try: {capture.lastSyncError}. It will try again automatically.
        {:else}
          It syncs as soon as the restaurant's server can be reached.
        {/if}
      </p>
    {/if}

    <div class="actions">
      {#if fromCapture}
        <a class="btn primary block tall" href="/capture">Capture the next plate</a>
        <a class="btn outline block" href="/">Done</a>
      {:else}
        <a class="btn primary block tall done" href="/history">Done</a>
      {/if}
    </div>
    <p class="helper center foot">
      {showQr ? "This invitation can expire or be revoked." : "Saved on this device first."}
    </p>
  {/if}
</main>

<style>
  .center {
    text-align: center;
  }
  .sep {
    margin: 0 0.6em;
  }
  .invite .display {
    margin-top: 10px;
  }
  .state-line {
    display: flex;
    justify-content: center;
    margin: 16px 0;
  }
  .state {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 0;
    font-size: 18px;
    font-weight: 600;
    letter-spacing: 0.01em;
  }
  .state.synced {
    color: var(--select);
  }
  .state.plain {
    color: var(--ok);
    font-size: 16px;
  }
  .badge {
    display: grid;
    place-items: center;
    width: 36px;
    height: 36px;
    border-radius: 50%;
    background: var(--select);
    color: var(--on-cta);
  }

  .dish {
    display: grid;
    grid-template-columns: 88px 1fr;
    align-items: center;
    gap: 18px;
    padding: 10px;
  }
  .dish .pic {
    height: 68px;
  }
  .dish-name {
    display: block;
    font-family: var(--serif);
    font-size: 21px;
    font-weight: 700;
    line-height: 1.15;
    color: var(--headline);
  }
  .where {
    display: block;
    margin-top: 8px;
    color: var(--muted);
  }
  .nowrap {
    white-space: nowrap;
  }
  .where .dot {
    margin: 0 0.8em;
  }

  .qr {
    width: min(70%, 280px);
    margin: 22px auto 0;
  }
  .concept {
    margin: 10px 0 0;
    text-align: center;
    font-size: 13px;
    letter-spacing: 0.14em;
    color: var(--muted);
  }
  .qr-wait {
    display: grid;
    justify-items: center;
    gap: 10px;
    width: min(64%, 260px);
    aspect-ratio: 1;
    margin: 22px auto 0;
    padding: 22px;
    place-content: center;
    border-radius: 18px;
    border: 1.5px dashed var(--edge);
    color: var(--wait);
    text-align: center;
  }
  .qr-wait p {
    margin: 0;
    color: var(--secondary);
    font-size: 14.5px;
  }
  .ask {
    margin: 18px 0 0;
    text-align: center;
    font-family: var(--serif);
    font-size: 19px;
    color: var(--text);
  }
  .ask.small {
    font-family: var(--sans);
    font-size: 14.5px;
    color: var(--secondary);
  }
  .actions {
    display: grid;
    gap: 10px;
    margin-top: 22px;
  }
  .done {
    font-family: var(--serif);
    font-size: 22px;
  }
  .foot {
    margin: 16px 0 0;
    letter-spacing: 0.06em;
  }
</style>
