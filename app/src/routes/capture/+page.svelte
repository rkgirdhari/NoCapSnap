<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import { goto, pushState, replaceState } from "$app/navigation";
  import { page } from "$app/state";
  import { bridge, type MenuItem, type Profile } from "$lib/bridge";
  import Icon from "$lib/components/Icon.svelte";
  import Thumb from "$lib/components/Thumb.svelte";
  import TopBar from "$lib/components/TopBar.svelte";
  import { TABLE_LABEL_MAX, searchKey, tableLabelError } from "$lib/format";
  import { cameraRoute, setCameraRoute, type CameraRoute } from "$lib/prefs";

  // Steps: prepare (01/03) → camera → review (02/03) → /invite (03/03).
  // Camera and review are shallow-routed so Android's back button walks them.
  const step = $derived(page.state.step ?? "prepare");

  // ---------- 01 / 03 Prepare ----------
  let profile = $state<Profile | null>(null);
  let menu = $state<MenuItem[]>([]);
  let lastPhoto = $state<Record<string, string>>({}); // menu item → latest capture digest
  let query = $state("");
  let category = $state("All");
  let selectedId = $state<string | null>(null);
  let tableLabel = $state("");
  let loadError = $state<string | null>(null);

  const categories = $derived(["All", ...new Set(menu.map((m) => m.category))]);
  const visible = $derived.by(() => {
    const q = searchKey(query);
    return menu.filter(
      (m) => (category === "All" || m.category === category) && (!q || searchKey(m.name).includes(q)),
    );
  });
  const selected = $derived(menu.find((m) => m.id === selectedId) ?? null);
  const labelError = $derived(tableLabelError(tableLabel));
  const cleanLabel = $derived(tableLabel.trim() || null);
  const menuIsDemo = $derived(menu.length > 0 && menu.every((m) => m.source === "demo"));
  const canContinue = $derived((menu.length === 0 || selected !== null) && !labelError);

  onMount(async () => {
    // Steps don't survive a reload or a return from another screen: start over.
    if (page.state.step) replaceState("", {});
    try {
      const [p, items, recent] = await Promise.all([bridge.profile(), bridge.menu(), bridge.list(200)]);
      profile = p;
      menu = items;
      const latest: Record<string, string> = {};
      for (const c of recent) if (c.menuItemId && !latest[c.menuItemId]) latest[c.menuItemId] = c.sha256;
      lastPhoto = latest;
    } catch (e) {
      loadError = message(e);
    }
  });

  function continueToCamera() {
    if (!canContinue) return;
    pushState("", { step: "camera" });
    // Still inside the tap, so the permission prompt belongs to it (Spec §3: ask at the moment of use).
    if (route === "viewfinder" && inAppCamera) startCamera();
  }

  // ---------- Camera ----------
  // Two routes, as compared in W1: the WebView viewfinder (getUserMedia) or the
  // phone's own camera app through a file input.
  let route = $state<CameraRoute>(cameraRoute());
  let live = $state(false);
  let cameraError = $state<string | null>(null);
  let video = $state<HTMLVideoElement>();
  let stream: MediaStream | null = null;
  // Bumped by every start and stop, so a camera that finishes starting after
  // the user has moved on (switched route, went back) is shut straight away.
  let startToken = 0;
  const inAppCamera = typeof navigator !== "undefined" && !!navigator.mediaDevices?.getUserMedia;

  function stopStream() {
    startToken++;
    stream?.getTracks().forEach((t) => t.stop());
    stream = null;
    live = false;
  }

  function describeCameraError(e: unknown): string {
    const name = e instanceof DOMException ? e.name : "";
    if (name === "NotAllowedError")
      return "Camera permission was declined. Allow it in Android settings, or use the phone camera app.";
    if (name === "NotFoundError") return "No camera was found on this device.";
    if (name === "NotReadableError") return "The camera is busy in another app. Close it and try again.";
    return message(e);
  }

  async function startCamera() {
    cameraError = null;
    const token = ++startToken;
    try {
      const started = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: { ideal: "environment" }, width: { ideal: 2048 }, height: { ideal: 1536 } },
        audio: false,
      });
      if (token !== startToken || route !== "viewfinder" || page.state.step !== "camera") {
        started.getTracks().forEach((t) => t.stop());
        return;
      }
      stream = started;
      live = true;
      await tick();
      if (video) {
        video.srcObject = stream;
        await video.play();
      }
    } catch (e) {
      if (token !== startToken) return;
      stopStream();
      cameraError = describeCameraError(e);
    }
  }

  async function takeFrame() {
    if (!video?.videoWidth) return;
    const canvas = document.createElement("canvas");
    canvas.width = video.videoWidth;
    canvas.height = video.videoHeight;
    canvas.getContext("2d")?.drawImage(video, 0, 0);
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/jpeg", 0.92));
    stopStream();
    if (blob) await usePhoto(blob);
    else cameraError = "The camera frame could not be captured. Try again.";
  }

  function onPicked(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) usePhoto(file);
  }

  function switchRoute() {
    stopStream();
    cameraError = null;
    route = route === "viewfinder" ? "phone" : "viewfinder";
    setCameraRoute(route);
  }

  // ---------- 02 / 03 Review ----------
  // The unprocessed photo stays in memory only; Rust processes it on save.
  let photo = $state<{ bytes: Uint8Array; url: string } | null>(null);
  let saving = $state(false);
  let saveError = $state<string | null>(null);

  function clearPhoto() {
    if (photo) URL.revokeObjectURL(photo.url);
    photo = null;
  }

  async function usePhoto(blob: Blob) {
    clearPhoto();
    saveError = null;
    photo = { bytes: new Uint8Array(await blob.arrayBuffer()), url: URL.createObjectURL(blob) };
    pushState("", { step: "review" });
  }

  // Set by Retake; the camera starts once the back navigation has landed on the camera step.
  let restartCamera = false;

  function retake() {
    restartCamera = route === "viewfinder" && inAppCamera;
    history.back(); // to the camera step, which clears the photo
  }

  async function save() {
    if (!photo || saving) return;
    saving = true;
    saveError = null;
    try {
      const capture = await bridge.ingest(photo.bytes, { menuItemId: selectedId, tableLabel: cleanLabel });
      clearPhoto();
      await goto(`/invite?c=${encodeURIComponent(capture.clientId)}&from=capture`, { replaceState: true });
    } catch (e) {
      saveError = message(e);
    } finally {
      saving = false;
    }
  }

  // ---------- Step bookkeeping ----------
  $effect(() => {
    if (step !== "camera" && live) stopStream(); // never leave the camera running off-screen
    if (step === "camera" && photo) clearPhoto(); // back from review = retake
    if (step === "camera" && restartCamera) {
      restartCamera = false;
      startCamera();
    }
  });

  function onVisibility() {
    if (document.hidden && live) stopStream();
  }
  $effect(() => {
    document.addEventListener("visibilitychange", onVisibility);
    return () => document.removeEventListener("visibilitychange", onVisibility);
  });

  onDestroy(() => {
    stopStream();
    clearPhoto();
  });

  const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
  const context = $derived(
    [profile?.locationName, cleanLabel ? `Table ${cleanLabel}` : null].filter((p): p is string => !!p),
  );
</script>

{#if step === "prepare"}
  <TopBar back="/" />
  <main class="page prepare">
    <p class="kicker">01 / 03 <span class="sep">·</span> Prepare</p>
    <h1 class="display">Which dish?</h1>
    {#if profile?.locationName}
      <p class="sub">{profile.locationName} · {menuIsDemo ? "Demo menu" : "Menu"}</p>
    {/if}

    <label class="search">
      <Icon name="search" size={24} stroke={1.5} />
      <span class="visually-hidden">Search the menu</span>
      <input type="search" placeholder="Search the menu" bind:value={query} autocomplete="off" />
    </label>

    {#if categories.length > 2}
      <div class="chips" role="group" aria-label="Course">
        {#each categories as c (c)}
          <button class="chip" aria-pressed={category === c} onclick={() => (category = c)}>{c}</button>
        {/each}
      </div>
    {/if}

    {#if loadError}
      <p class="error" role="alert">{loadError}</p>
    {/if}

    <fieldset class="dishes">
      <legend class="visually-hidden">Dish</legend>
      {#each visible as item (item.id)}
        <label class="dish" class:on={selectedId === item.id}>
          <input class="visually-hidden" type="radio" name="dish" value={item.id} bind:group={selectedId} />
          <span class="pic"><Thumb sha256={lastPhoto[item.id]} alt="" /></span>
          <span class="dish-name">{item.name}</span>
          <span class="radio" aria-hidden="true">
            {#if selectedId === item.id}<Icon name="check" size={20} stroke={2.4} />{/if}
          </span>
        </label>
      {:else}
        {#if menu.length}
          <p class="helper">No dish matches “{query.trim()}”.</p>
        {:else if !loadError}
          <p class="helper">No menu on this device yet. You can still capture the plate.</p>
        {/if}
      {/each}
    </fieldset>

    <label class="field">
      <span class="field-label">Table label (optional)</span>
      <input
        type="text"
        placeholder="e.g. 12B"
        maxlength={TABLE_LABEL_MAX}
        autocomplete="off"
        autocapitalize="characters"
        bind:value={tableLabel}
        aria-invalid={labelError ? "true" : undefined}
        aria-describedby="table-help"
      />
      <span id="table-help" class="helper" class:bad={labelError}>
        {labelError ?? "For staff reference only · stays on this device"}
      </span>
    </label>

    <div class="dock">
      <button class="btn primary block tall" onclick={continueToCamera} disabled={!canContinue}>
        <Icon name="camera" size={28} stroke={1.7} /> Continue to camera
      </button>
    </div>
  </main>
{:else if step === "camera"}
  <TopBar back={() => history.back()} />
  <main class="page camera">
    <p class="kicker">{selected?.name ?? "New capture"}{#if cleanLabel}&ensp;·&ensp;Table {cleanLabel}{/if}</p>

    <div class="frame">
      {#if live}
        <!-- svelte-ignore a11y_media_has_caption -->
        <video bind:this={video} playsinline muted></video>
        <button class="shutter" onclick={takeFrame} aria-label="Take photo"><span></span></button>
      {:else}
        <div class="empty">
          <svg class="guide" viewBox="0 0 100 100" aria-hidden="true">
            <circle cx="50" cy="50" r="42" /><circle cx="50" cy="50" r="30" />
          </svg>
          {#if route === "viewfinder"}
            <p>CapSnap uses the camera only while you frame the plate.</p>
            <button class="btn primary" onclick={startCamera} disabled={!inAppCamera}>
              <Icon name="camera" /> Start camera
            </button>
            {#if !inAppCamera}
              <p class="helper">No in-app camera here. Use the phone camera app instead.</p>
            {/if}
          {:else}
            <p>Opens your phone's camera app, then brings the photo back here.</p>
            <label class="btn primary picker">
              <Icon name="camera" /> Open camera app
              <input class="visually-hidden" type="file" accept="image/*" capture="environment" onchange={onPicked} />
            </label>
          {/if}
        </div>
      {/if}
    </div>

    {#if cameraError}
      <p class="error" role="alert">{cameraError}</p>
    {/if}

    <button class="switch" onclick={switchRoute}>
      {route === "viewfinder" ? "Use the phone camera app instead" : "Use the in-app viewfinder instead"}
    </button>
  </main>
{:else if step === "review"}
  <TopBar back={() => history.back()} />
  <main class="page review">
    <p class="kicker center">02 / 03 <span class="sep">·</span> Review</p>
    <h1 class="display center">The plate, as served.</h1>

    <div class="shot">
      {#if photo}
        <!-- Shown whole (not cropped) so nothing at the edges escapes the check. -->
        <img src={photo.url} alt="The plate you just photographed" />
      {/if}
    </div>

    <div class="card what">
      <p class="what-dish">{selected?.name ?? "Unlabelled dish"}</p>
      {#if context.length}
        <p class="caps what-where">
          {#each context as part, i (part)}{#if i}<span class="dot">·</span>{/if}<span class="nowrap">{part}</span>{/each}
        </p>
      {/if}
    </div>

    <p class="hint"><Icon name="info" size={22} stroke={1.4} /> Check the dish. Keep diners and receipts out of frame.</p>

    {#if saveError}
      <p class="error" role="alert">{saveError}</p>
    {/if}

    <div class="pair">
      <button class="btn outline tall" onclick={retake} disabled={saving}>
        <Icon name="sync" size={22} stroke={1.6} /> Retake
      </button>
      <button class="btn primary tall" onclick={save} disabled={saving || !photo}>
        <Icon name="save" size={22} stroke={1.7} />
        {saving ? "Saving…" : "Save this capture"}
      </button>
    </div>
    <p class="helper center">Saved on this device first.</p>
  </main>
{/if}

<style>
  .sep {
    margin: 0 0.6em;
  }
  .center {
    text-align: center;
  }

  /* ---- Prepare ---- */
  .prepare .display {
    margin-top: 6px;
  }
  .sub {
    margin: 6px 0 0;
    font-size: 15.5px;
    letter-spacing: 0.04em;
    color: var(--muted);
  }

  .search {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-top: 20px;
    padding: 0 14px;
    min-height: var(--tap);
    border-radius: var(--radius);
    background: var(--input);
    border: 1px solid var(--edge);
    color: var(--on-input);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    outline: none;
    font-size: 16px;
    letter-spacing: 0.03em;
    color: var(--text);
  }
  .search:focus-within {
    outline: 2px solid var(--select);
    outline-offset: 2px;
  }
  input::placeholder {
    color: var(--on-input);
  }

  .chips {
    display: flex;
    gap: 8px;
    margin-top: 14px;
    overflow-x: auto;
  }
  .chip {
    flex: none;
    min-height: 44px;
    padding: 0 20px;
    border-radius: 999px;
    border: 1.5px solid transparent;
    background: var(--card);
    color: var(--text);
    font-size: 15px;
    letter-spacing: 0.02em;
    cursor: pointer;
  }
  .chip[aria-pressed="true"] {
    border-color: var(--select);
    background: var(--wait-bg);
  }

  .dishes {
    display: grid;
    gap: 10px;
    margin: 14px 0 0;
    padding: 0;
    border: 0;
  }
  .dish {
    display: grid;
    grid-template-columns: 76px 1fr 32px;
    align-items: center;
    gap: 14px;
    padding: 7px 14px 7px 7px;
    border-radius: var(--radius);
    background: var(--card);
    border: 1.5px solid var(--hairline);
    cursor: pointer;
  }
  .dish.on {
    border-color: var(--select);
  }
  .dish:focus-within {
    outline: 2px solid var(--select);
    outline-offset: 2px;
  }
  .dish .pic {
    height: 58px;
  }
  .dish-name {
    font-family: var(--serif);
    font-size: 16.5px;
    font-weight: 700;
    line-height: 1.2;
    color: var(--headline);
  }
  .radio {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    border: 1.5px solid var(--muted);
    color: var(--on-cta);
  }
  .on .radio {
    border-color: var(--select);
    background: var(--select);
  }

  .field {
    display: grid;
    gap: 8px;
    margin-top: 22px;
  }
  .field-label {
    font-size: 16px;
    letter-spacing: 0.02em;
  }
  .field input {
    min-height: var(--tap);
    padding: 0 16px;
    border-radius: var(--radius);
    background: var(--input);
    border: 1px solid var(--edge);
    font-size: 17px;
    letter-spacing: 0.06em;
  }
  .field .bad {
    color: var(--danger);
  }

  /* The next step stays reachable above the tab bar however long the menu is. */
  .dock {
    position: sticky;
    bottom: calc(var(--tabbar) + var(--safe-bottom) + 10px);
    margin-top: 18px;
    padding-top: 12px;
    background: linear-gradient(transparent, var(--bg) 40%);
  }

  /* ---- Camera ---- */
  .frame {
    position: relative;
    margin-top: 14px;
    aspect-ratio: 3 / 4;
    max-height: calc(100dvh - 230px);
    width: 100%;
    overflow: hidden;
    border-radius: var(--radius-lg);
    background: radial-gradient(circle at 50% 40%, #2c1c29, #140a19 75%);
    border: 1px solid var(--hairline);
  }
  .frame video {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
  .empty {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 28px;
    text-align: center;
    color: var(--secondary);
  }
  .empty p {
    margin: 0;
    max-width: 28ch;
  }
  .guide {
    width: 110px;
    fill: none;
    stroke: var(--select);
    stroke-width: 1.4;
  }
  .guide circle + circle {
    stroke: var(--muted);
    stroke-width: 1;
  }
  .picker:focus-within {
    outline: 2px solid var(--select);
    outline-offset: 3px;
  }
  .shutter {
    position: absolute;
    left: 50%;
    bottom: 22px;
    translate: -50% 0;
    display: grid;
    place-items: center;
    width: 84px;
    height: 84px;
    padding: 0;
    border-radius: 50%;
    border: 4px solid var(--headline);
    background: rgba(0, 0, 0, 0.18);
    cursor: pointer;
  }
  .shutter span {
    width: 64px;
    height: 64px;
    border-radius: 50%;
    background: var(--cta);
    transition: scale 90ms ease;
  }
  .shutter:active span {
    scale: 0.9;
  }
  .switch {
    display: block;
    margin: 16px auto 0;
    min-height: 44px;
    padding: 0 12px;
    border: 0;
    background: none;
    color: var(--text);
    font-size: 15px;
    text-decoration: underline;
    text-underline-offset: 4px;
    cursor: pointer;
  }

  /* ---- Review ---- */
  .review .display {
    margin-top: 10px;
    font-size: clamp(30px, 9.2vw, 40px);
  }
  .shot {
    display: grid;
    place-items: center;
    margin-top: 18px;
    min-height: 200px;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: #0f0714;
    border: 1px solid var(--hairline);
  }
  .shot img {
    width: 100%;
    max-height: 40dvh;
    object-fit: contain;
  }
  .what {
    margin-top: 14px;
    padding: 16px 18px;
  }
  .what p {
    margin: 0;
  }
  .what-dish {
    font-family: var(--serif);
    font-size: 23px;
    font-weight: 700;
    line-height: 1.15;
    color: var(--headline);
  }
  .what-where {
    margin-top: 8px !important;
    color: var(--muted);
  }
  .nowrap {
    white-space: nowrap;
  }
  .what-where .dot {
    margin: 0 0.8em;
  }
  .hint {
    display: flex;
    align-items: center;
    gap: 12px;
    margin: 14px 4px;
    font-size: 13.5px;
    letter-spacing: 0.02em;
    color: var(--text);
  }
  .hint :global(svg) {
    flex: none;
    color: var(--muted);
  }
  .pair {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 10px;
  }
  .pair .btn {
    padding: 0 16px;
    gap: 8px;
    font-size: 15.5px;
    white-space: nowrap;
  }
  .review .helper {
    margin: 14px 0 0;
  }
</style>
