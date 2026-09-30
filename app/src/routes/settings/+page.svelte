<script lang="ts">
  import { onMount } from "svelte";
  import { bridge, type AppInfo, type MenuItem, type Profile, type StoreStatus } from "$lib/bridge";
  import TopBar from "$lib/components/TopBar.svelte";
  import { cameraRoute, setCameraRoute, type CameraRoute } from "$lib/prefs";

  let info = $state<AppInfo | null>(null);
  let profile = $state<Profile | null>(null);
  let menu = $state<MenuItem[]>([]);
  let status = $state<StoreStatus | null>(null);
  let name = $state("");
  let nameNote = $state<string | null>(null);
  let route = $state<CameraRoute>(cameraRoute());
  let check = $state<{ ok: boolean; text: string } | null>(null);
  let checking = $state(false);
  let ackNote = $state<string | null>(null);
  let error = $state<string | null>(null);

  const message = (e: unknown) => (e instanceof Error ? e.message : String(e));

  onMount(async () => {
    try {
      [info, profile, menu, status] = await Promise.all([
        bridge.appInfo(),
        bridge.profile(),
        bridge.menu(),
        bridge.status(),
      ]);
      name = profile.displayName ?? "";
    } catch (e) {
      error = message(e);
    }
  });

  async function saveName(event: SubmitEvent) {
    event.preventDefault();
    nameNote = null;
    try {
      profile = await bridge.setDisplayName(name);
      name = profile.displayName ?? "";
      nameNote = "Saved on this device.";
    } catch (e) {
      nameNote = message(e);
    }
  }

  function chooseRoute(next: CameraRoute) {
    route = next;
    setCameraRoute(next);
  }

  async function runCheck() {
    checking = true;
    check = null;
    try {
      const text = await bridge.selftest();
      status = await bridge.status();
      check = { ok: true, text };
    } catch (e) {
      check = { ok: false, text: message(e) };
    } finally {
      checking = false;
    }
  }

  async function simulateAck() {
    ackNote = null;
    try {
      const acked = await bridge.simulateAck();
      status = await bridge.status();
      ackNote = acked ? `Marked synced: ${acked.dishName ?? "unlabelled dish"}.` : "Nothing is waiting.";
    } catch (e) {
      ackNote = message(e);
    }
  }
</script>

<TopBar status />

<main class="page settings">
  <h1 class="display">Settings.</h1>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  <section>
    <h2 class="kicker">You</h2>
    <form class="card pad" onsubmit={saveName}>
      <label class="field">
        <span>Name for the greeting</span>
        <input type="text" maxlength="40" autocomplete="given-name" bind:value={name} placeholder="e.g. Maya" />
      </label>
      <button class="btn outline" type="submit">Save name</button>
      <p class="note muted">{nameNote ?? "Stored on this device only."}</p>
    </form>
  </section>

  <section>
    <h2 class="kicker">Restaurant</h2>
    <div class="card pad">
      <p class="big">{profile?.locationName ?? "No location"}{#if profile?.isDemo}<span class="demo">Demo</span>{/if}</p>
      <p class="muted">
        {menu.length} dishes · {menu.every((m) => m.source === "demo") ? "demo menu" : "menu"}.
        {#if profile?.isDemo}Sample location and menu until this phone signs in to your restaurant.{/if}
      </p>
    </div>
  </section>

  <section>
    <h2 class="kicker">Camera</h2>
    <div class="card pad" role="radiogroup" aria-label="Camera">
      <label class="opt">
        <input type="radio" name="route" checked={route === "viewfinder"} onchange={() => chooseRoute("viewfinder")} />
        <span><strong>In-app viewfinder</strong><span class="muted">Frame the plate without leaving CapSnap.</span></span>
      </label>
      <label class="opt">
        <input type="radio" name="route" checked={route === "phone"} onchange={() => chooseRoute("phone")} />
        <span><strong>Phone camera app</strong><span class="muted">Your camera app, then back to CapSnap.</span></span>
      </label>
    </div>
  </section>

  <section>
    <h2 class="kicker">Privacy</h2>
    <div class="card pad">
      <p class="muted">
        Before a photo is saved, this phone resizes it to 2048 px and removes its location and camera data. No guest
        names, emails or phone numbers are ever collected.
      </p>
    </div>
  </section>

  <section>
    <h2 class="kicker">Device check</h2>
    <div class="card pad">
      <button class="btn outline" onclick={runCheck} disabled={checking}>
        {checking ? "Checking…" : "Run device check"}
      </button>
      {#if check}
        <p class="note mono" class:bad={!check.ok} role="status">{check.text}</p>
      {/if}
      {#if status}
        <dl class="facts">
          <dt>Saved offline</dt><dd>{status.pending}</dd>
          <dt>Synced</dt><dd>{status.synced}</dd>
          <dt>SQLite</dt><dd class="mono">{status.sqliteVersion} · {status.journalMode}</dd>
        </dl>
      {/if}
    </div>
  </section>

  {#if info?.debug}
    <section>
      <h2 class="kicker">Developer · debug build only</h2>
      <div class="card pad">
        <p class="muted">Stands in for the server acknowledging the oldest waiting plate, to review the “QR ready” screens.</p>
        <button class="btn outline" onclick={simulateAck}>Simulate server acknowledgement</button>
        {#if ackNote}<p class="note muted" role="status">{ackNote}</p>{/if}
      </div>
    </section>
  {/if}

  <p class="helper about">
    CapSnap {info?.version ?? ""}{bridge.mode === "preview" ? " · browser preview" : ""} · Hammurabi Coding Company LLC
  </p>
</main>

<style>
  .settings .display {
    margin-top: 8px;
  }
  section {
    margin-top: 24px;
  }
  section .kicker {
    margin-bottom: 10px;
  }
  .pad {
    display: grid;
    gap: 12px;
    padding: 16px;
  }
  .pad p {
    margin: 0;
  }
  .big {
    display: flex;
    align-items: center;
    gap: 10px;
    font-family: var(--serif);
    font-size: 22px;
    font-weight: 700;
    color: var(--headline);
  }
  .demo {
    padding: 2px 7px;
    border-radius: 5px;
    background: var(--wait-bg);
    color: var(--wait);
    font-family: var(--sans);
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }
  .field {
    display: grid;
    gap: 8px;
  }
  .field input {
    min-height: var(--tap);
    padding: 0 14px;
    border-radius: var(--radius);
    background: var(--input);
    border: 1px solid var(--edge);
    font-size: 17px;
  }
  .field input::placeholder {
    color: var(--on-input);
  }
  .opt {
    display: grid;
    grid-template-columns: 24px 1fr;
    gap: 12px;
    align-items: start;
    min-height: 44px;
    cursor: pointer;
  }
  .opt input {
    width: 20px;
    height: 20px;
    margin: 2px 0 0;
    accent-color: var(--select);
  }
  .opt strong {
    display: block;
    font-weight: 600;
  }
  .opt .muted {
    font-size: 14.5px;
  }
  .note {
    font-size: 14px;
  }
  .note.bad {
    color: var(--danger);
  }
  .facts {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 6px 16px;
    margin: 0;
    font-size: 14.5px;
  }
  .facts dt {
    color: var(--muted);
  }
  .facts dd {
    margin: 0;
  }
  .about {
    margin: 28px 0 0;
    text-align: center;
  }
</style>
