<script lang="ts">
  import { bridge, type StoreStatus } from "$lib/bridge";

  let status = $state<StoreStatus | null>(null);
  let selftest = $state<string | null>(null);
  let ackNote = $state<string | null>(null);
  let busy = $state(false);

  const inAppCamera = typeof navigator !== "undefined" && !!navigator.mediaDevices?.getUserMedia;
  const webview = typeof navigator === "undefined" ? "—" : (navigator.userAgent.match(/Chrome\/[\d.]+/)?.[0] ?? navigator.userAgent);

  async function refresh() {
    status = await bridge.status();
  }

  async function runSelftest() {
    busy = true;
    try {
      selftest = await bridge.selftest();
    } catch (e) {
      selftest = `failed: ${e instanceof Error ? e.message : String(e)}`;
    } finally {
      busy = false;
    }
  }

  async function simulateAck() {
    busy = true;
    try {
      const acked = await bridge.simulateAck();
      ackNote = acked ? "Oldest waiting plate marked as synced." : "Nothing is waiting to sync.";
      await refresh();
    } finally {
      busy = false;
    }
  }

  $effect(() => {
    refresh();
  });
</script>

<p class="kicker">Manager</p>
<h1 class="display">Back of house.</h1>
<p class="lede">Menu, staff and guest feedback will live here once the server is running.</p>

<ul class="card rows">
  <li><span>Menu &amp; dishes</span><span class="tag">Phase 3</span></li>
  <li><span>Staff &amp; roles</span><span class="tag">Phase 3</span></li>
  <li><span>Guest feedback</span><span class="tag">Phase 3</span></li>
</ul>

<hr class="rule" />

<p class="kicker">Device check · W1</p>
<section class="card">
  <dl>
    <dt>Storage</dt>
    <dd>
      {#if bridge.mode === "preview"}
        On the device only
      {:else}
        {status ? `SQLite ${status.sqliteVersion} · ${status.journalMode.toUpperCase()} journal` : "…"}
      {/if}
    </dd>
    <dt>Captures</dt>
    <dd>{status ? `${status.pending} waiting · ${status.synced} QR ready` : "…"}</dd>
    <dt>In-app camera</dt>
    <dd>{inAppCamera ? "Available" : "Not available in this WebView"}</dd>
    <dt>WebView</dt>
    <dd class="mono">{webview}</dd>
  </dl>

  <div class="tools">
    <button class="btn quiet block" onclick={runSelftest} disabled={busy}>Run storage self-test</button>
    {#if selftest}<p class="mono result" role="status">{selftest}</p>{/if}
    <button class="btn quiet block" onclick={simulateAck} disabled={busy}>
      Simulate server sync
    </button>
    <p class="muted small">
      Spike only: stands in for the server so the "Synced · QR ready" state can be checked on the device.
    </p>
    {#if ackNote}<p class="result" role="status">{ackNote}</p>{/if}
  </div>
</section>

<p class="colophon">Hammurabi Coding Company LLC · Founder R. K. Girdhari</p>

<style>
  .rows {
    list-style: none;
    margin: 24px 0 0;
    padding: 0 18px;
  }

  .rows li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-height: 56px;
    font-weight: 600;
    color: var(--ink-soft);
  }

  .rows li + li {
    border-top: 1px solid var(--hairline);
  }

  .tag {
    padding: 4px 10px;
    border-radius: 999px;
    border: 1px solid color-mix(in srgb, var(--gold) 60%, transparent);
    color: var(--walnut);
    font-size: 12px;
    letter-spacing: 0.06em;
  }

  dl {
    display: grid;
    grid-template-columns: 112px 1fr;
    gap: 12px 14px;
    margin: 0 0 18px;
  }

  dt {
    color: var(--ink-muted);
    font-size: 13.5px;
    font-weight: 600;
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  .tools {
    display: grid;
    gap: 10px;
  }

  .tools p {
    margin: 0;
  }

  .small {
    font-size: 13px;
  }

  .result {
    padding: 10px 12px;
    border-radius: 10px;
    background: var(--paper-sunk);
    font-size: 13.5px;
  }

  .colophon {
    margin: 34px 0 0;
    text-align: center;
    color: var(--ink-muted);
    font-size: 12.5px;
    letter-spacing: 0.04em;
  }
</style>
