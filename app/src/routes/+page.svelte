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

<section class="ledger on-surface" aria-label="Tonight at a glance" aria-busy={!loaded}>
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
    border: 1px solid var(--surface-edge);
    border-radius: var(--radius);
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.05), transparent 40%), var(--surface);
    box-shadow: var(--shadow);
  }

  .tile {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 18px 14px 16px;
  }

  .tile + .tile {
    border-left: 1px solid color-mix(in srgb, var(--muted-gold) 70%, transparent);
  }

  .figure {
    font-family: var(--serif);
    font-size: 38px;
    line-height: 1;
    font-variant-numeric: lining-nums tabular-nums;
  }

  .figure.pending {
    color: var(--accent);
  }

  .figure.synced {
    color: var(--success); /* 4.47:1 on bamboo: passes as large text only, which it is */
  }

  .label {
    color: var(--on-surface-muted);
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
    border-left: 2px solid var(--muted-gold);
    color: var(--text-soft);
    font-size: 14px;
    background: var(--bg-deep);
    border-radius: 0 10px 10px 0;
  }
</style>
