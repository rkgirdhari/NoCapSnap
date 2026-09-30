<script lang="ts">
  import { bridge, type CaptureRecord } from "$lib/bridge";
  import { isSameLocalDay, serviceLine } from "$lib/format";

  let captures = $state<CaptureRecord[]>([]);
  let loaded = $state(false);

  const today = $derived(captures.filter((c) => isSameLocalDay(new Date(c.capturedAt), new Date())).length);
  const waiting = $derived(captures.filter((c) => c.syncState === "pending").length);
  const synced = $derived(captures.filter((c) => c.syncState === "synced").length);

  $effect(() => {
    bridge
      .list()
      .then((list) => (captures = list))
      .finally(() => (loaded = true));
  });
</script>

<p class="kicker">{serviceLine()}</p>
<h1 class="display">Every plate,<br />as it left the pass.</h1>
<p class="lede">
  Photograph the dish before it goes out. It stays on this phone until the server confirms it —
  then the guest's QR is ready.
</p>

<hr class="rule" />

<section class="ledger" aria-label="Tonight at a glance" aria-busy={!loaded}>
  <div class="tile">
    <span class="figure">{today}</span>
    <span class="label">Plated today</span>
  </div>
  <div class="tile">
    <span class="figure pending">{waiting}</span>
    <span class="label">Waiting to sync</span>
  </div>
  <div class="tile">
    <span class="figure synced">{synced}</span>
    <span class="label">QR ready</span>
  </div>
</section>

<a class="btn primary block cta" href="/capture">Capture a plate</a>

<aside class="note">
  <strong>W1 build.</strong> Sign-in and syncing arrive in later gates, so every capture stays on
  this phone for now.
</aside>

<style>
  .ledger {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    border: 1px solid var(--hairline);
    border-radius: var(--radius);
    background: var(--paper-raised);
    box-shadow: var(--shadow);
  }

  .tile {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 18px 14px 16px;
  }

  .tile + .tile {
    border-left: 1px solid var(--hairline);
  }

  .figure {
    font-family: var(--serif);
    font-size: 38px;
    line-height: 1;
    font-variant-numeric: lining-nums tabular-nums;
  }

  .figure.pending {
    color: var(--indigo);
  }

  .figure.synced {
    color: var(--matcha);
  }

  .label {
    color: var(--ink-muted);
    font-size: 12.5px;
    font-weight: 600;
    letter-spacing: 0.02em;
  }

  .cta {
    margin-top: 22px;
  }

  .note {
    margin-top: 22px;
    padding: 14px 16px;
    border-left: 2px solid var(--gold);
    color: var(--ink-soft);
    font-size: 14px;
    background: color-mix(in srgb, var(--paper-sunk) 55%, transparent);
    border-radius: 0 10px 10px 0;
  }
</style>
