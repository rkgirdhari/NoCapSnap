<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import { bridge, type CaptureRecord } from "$lib/bridge";
  import Icon from "$lib/components/Icon.svelte";
  import Stamp from "$lib/components/Stamp.svelte";
  import StatusPill from "$lib/components/StatusPill.svelte";
  import { formatBytes, formatTime, shortDigest } from "$lib/format";

  // W1 compares both Android camera routes on a real device (see the W1 gate report):
  // "viewfinder" = WebView getUserMedia, "phone" = the system camera app via a file input.
  type Mode = "viewfinder" | "phone";
  type Phase = "idle" | "live" | "review" | "saving" | "saved";

  let mode = $state<Mode>("viewfinder");
  let phase = $state<Phase>("idle");
  let error = $state<string | null>(null);
  let photo = $state<{ bytes: Uint8Array; url: string } | null>(null);
  let saved = $state<CaptureRecord | null>(null);
  let video = $state<HTMLVideoElement>();
  let stream: MediaStream | null = null;

  const inAppCamera = typeof navigator !== "undefined" && !!navigator.mediaDevices?.getUserMedia;

  function stopStream() {
    stream?.getTracks().forEach((track) => track.stop());
    stream = null;
  }

  function clearPhoto() {
    if (photo) URL.revokeObjectURL(photo.url);
    photo = null;
  }

  function describeCameraError(e: unknown): string {
    const name = e instanceof DOMException ? e.name : "";
    if (name === "NotAllowedError")
      return "Camera permission was declined. Allow it in Android settings, or switch to Phone camera.";
    if (name === "NotFoundError") return "No camera was found on this device.";
    if (name === "NotReadableError") return "The camera is busy in another app. Close it and try again.";
    return e instanceof Error ? e.message : String(e);
  }

  // Camera access is requested only here, at the moment of use (Spec §3).
  async function startCamera() {
    error = null;
    try {
      stream = await navigator.mediaDevices.getUserMedia({
        video: { facingMode: { ideal: "environment" }, width: { ideal: 2048 }, height: { ideal: 1536 } },
        audio: false,
      });
      phase = "live";
      await tick();
      if (video) {
        video.srcObject = stream;
        await video.play();
      }
    } catch (e) {
      stopStream();
      phase = "idle";
      error = describeCameraError(e);
    }
  }

  async function takeFrame() {
    if (!video || !video.videoWidth) return;
    const canvas = document.createElement("canvas");
    canvas.width = video.videoWidth;
    canvas.height = video.videoHeight;
    canvas.getContext("2d")?.drawImage(video, 0, 0);
    const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, "image/jpeg", 0.9));
    stopStream();
    if (blob) await usePhoto(blob);
    else {
      phase = "idle";
      error = "The camera frame could not be captured. Try again.";
    }
  }

  async function usePhoto(blob: Blob) {
    clearPhoto();
    photo = { bytes: new Uint8Array(await blob.arrayBuffer()), url: URL.createObjectURL(blob) };
    phase = "review";
  }

  function onPicked(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (file) usePhoto(file);
  }

  async function save() {
    if (!photo) return;
    phase = "saving";
    error = null;
    try {
      saved = await bridge.ingest(photo.bytes);
      phase = "saved";
    } catch (e) {
      phase = "review";
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function again() {
    clearPhoto();
    saved = null;
    error = null;
    phase = "idle";
    if (mode === "viewfinder" && inAppCamera) startCamera();
  }

  function switchMode(next: Mode) {
    if (next === mode) return;
    stopStream();
    clearPhoto();
    saved = null;
    error = null;
    phase = "idle";
    mode = next;
  }

  // Never leave the camera running behind the app's back.
  function onVisibility() {
    if (document.hidden && phase === "live") {
      stopStream();
      phase = "idle";
    }
  }

  $effect(() => {
    document.addEventListener("visibilitychange", onVisibility);
    return () => document.removeEventListener("visibilitychange", onVisibility);
  });

  onDestroy(() => {
    stopStream();
    clearPhoto();
  });
</script>

{#if phase === "idle"}
  <p class="kicker">New capture</p>
  <h1 class="display">Frame the plate.</h1>
  <p class="lede">Straight on or three-quarter — the way the guest will first see it.</p>

  <div class="segment" role="tablist" aria-label="Camera">
    <button role="tab" aria-selected={mode === "viewfinder"} onclick={() => switchMode("viewfinder")}>
      In-app viewfinder
    </button>
    <button role="tab" aria-selected={mode === "phone"} onclick={() => switchMode("phone")}>
      Phone camera
    </button>
  </div>
{:else}
  <!-- Once the camera is up, the photo gets the screen. -->
  <p class="kicker">
    New capture · {mode === "viewfinder" ? "In-app viewfinder" : "Phone camera"}
  </p>
{/if}

<div class="frame phase-{phase}">
  {#if photo}
    <img src={photo.url} alt="The plate you just photographed" />
    {#if phase === "saved"}
      <div class="stamp-at"><Stamp /></div>
    {/if}
  {:else if phase === "live"}
    <!-- svelte-ignore a11y_media_has_caption -->
    <video bind:this={video} playsinline muted></video>
    <div class="scrim" aria-hidden="true"></div>
    <button class="shutter" onclick={takeFrame} aria-label="Take photo">
      <svg viewBox="0 0 100 100" aria-hidden="true">
        <circle class="ring" cx="50" cy="50" r="41" pathLength="100" />
        <circle class="core" cx="50" cy="50" r="22" />
      </svg>
    </button>
  {:else}
    <div class="empty">
      <!-- A plate seen from above: where the dish goes. -->
      <svg class="guide" viewBox="0 0 100 100" aria-hidden="true">
        <circle cx="50" cy="50" r="42" />
        <circle cx="50" cy="50" r="31" />
      </svg>
      {#if mode === "viewfinder"}
        <p>CapSnap asks for the camera only now, while you're plating.</p>
        <button class="btn primary" onclick={startCamera} disabled={!inAppCamera}>
          <Icon name="camera" /> Start camera
        </button>
        {#if !inAppCamera}
          <p class="muted small">This screen has no in-app camera. Use Phone camera instead.</p>
        {/if}
      {:else}
        <p>Opens your phone's own camera app, then brings the photo back here.</p>
        <label class="btn primary picker">
          <Icon name="camera" /> Open camera app
          <input class="visually-hidden" type="file" accept="image/*" capture="environment" onchange={onPicked} />
        </label>
      {/if}
    </div>
  {/if}
  <span class="corner tl" aria-hidden="true"></span>
  <span class="corner tr" aria-hidden="true"></span>
  <span class="corner bl" aria-hidden="true"></span>
  <span class="corner br" aria-hidden="true"></span>
</div>

{#if error}
  <p class="error" role="alert">{error}</p>
{/if}

<div class="actions">
  {#if phase === "live"}
    <button class="btn quiet" onclick={() => switchMode(mode === "viewfinder" ? "phone" : "viewfinder")}>
      Use the phone camera instead
    </button>
  {:else if phase === "review" || phase === "saving"}
    <div class="pair">
      <button class="btn quiet" onclick={again} disabled={phase === "saving"}>Retake</button>
      <button class="btn primary" onclick={save} disabled={phase === "saving"}>
        {phase === "saving" ? "Saving…" : "Save plate"}
      </button>
    </div>
  {:else if phase === "saved" && saved}
    <div class="receipt">
      <StatusPill state={saved.syncState} />
      <p class="mono muted">
        #{shortDigest(saved.sha256)} · {formatBytes(saved.bytes)} · {formatTime(saved.capturedAt)}
      </p>
    </div>
    <button class="btn primary block" onclick={again}>Capture the next plate</button>
    <a class="btn quiet block" href="/history">See today's plates</a>
  {/if}
</div>

<style>
  .segment {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 4px;
    margin: 24px 0 16px;
    padding: 4px;
    border-radius: 12px;
    background: var(--bg-deep);
    border: 1px solid var(--line);
  }

  .segment button {
    min-height: 44px;
    border: 0;
    border-radius: 9px;
    background: transparent;
    color: var(--text-muted);
    font: 600 14.5px/1 var(--sans);
    cursor: pointer;
  }

  .segment button[aria-selected="true"] {
    background: var(--surface);
    color: var(--text);
    box-shadow: 0 1px 0 rgba(246, 224, 94, 0.18) inset;
  }

  .frame {
    position: relative;
    aspect-ratio: 4 / 5;
    overflow: hidden;
    border-radius: 18px;
    background: linear-gradient(180deg, rgba(255, 255, 255, 0.05), transparent 40%), var(--surface);
    border: 1px solid var(--surface-edge);
  }

  /* Keep the next action above the tab bar on a phone screen. */
  .frame.phase-live {
    max-height: calc(100dvh - 250px);
  }
  .frame.phase-review,
  .frame.phase-saving {
    max-height: calc(100dvh - 330px);
  }
  .frame.phase-saved {
    max-height: calc(100dvh - 440px);
  }

  .frame img,
  .frame video {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  /* Gold corner ticks, like the marks on a framing card. */
  .corner {
    position: absolute;
    width: 22px;
    height: 22px;
    border-color: var(--accent);
    border-style: solid;
    border-width: 0;
    opacity: 0.9;
    pointer-events: none;
  }
  .tl { top: 12px; left: 12px; border-top-width: 2px; border-left-width: 2px; }
  .tr { top: 12px; right: 12px; border-top-width: 2px; border-right-width: 2px; }
  .bl { bottom: 12px; left: 12px; border-bottom-width: 2px; border-left-width: 2px; }
  .br { bottom: 12px; right: 12px; border-bottom-width: 2px; border-right-width: 2px; }

  .empty {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 16px;
    padding: 32px;
    text-align: center;
    color: var(--text-soft);
  }

  .empty p {
    margin: 0;
    max-width: 28ch;
  }

  .small {
    font-size: 13px;
  }

  .guide {
    width: 104px;
    height: 104px;
  }

  .guide circle {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.6;
  }

  .guide circle + circle {
    stroke: var(--text);
    stroke-width: 1;
    opacity: 0.3;
  }

  .scrim {
    position: absolute;
    inset: auto 0 0 0;
    height: 150px;
    background: linear-gradient(transparent, rgba(12, 9, 7, 0.55));
    pointer-events: none;
  }

  .picker:focus-within {
    outline: 2px solid var(--accent);
    outline-offset: 3px;
  }

  .stamp-at {
    position: absolute;
    right: 22px;
    bottom: 26px;
  }

  .error {
    margin: 14px 0 0;
    padding: 12px 14px;
    border-radius: 10px;
    background: var(--danger-tint);
    color: var(--danger);
    font-size: 14.5px;
    font-weight: 600;
  }

  .actions {
    display: grid;
    gap: 12px;
    justify-items: center;
    margin-top: 22px;
  }

  .pair {
    display: grid;
    grid-template-columns: 1fr 1.4fr;
    gap: 12px;
    width: 100%;
  }

  /* The shutter is an ensō: one confident gold stroke around a gold centre,
     floating over the viewfinder like a camera app's. */
  .shutter {
    position: absolute;
    left: 50%;
    bottom: 22px;
    translate: -50% 0;
    z-index: 1;
    width: 88px;
    height: 88px;
    padding: 0;
    border: 0;
    border-radius: 50%;
    background: transparent;
    cursor: pointer;
    transition: transform 90ms ease;
  }
  .shutter:active {
    scale: 0.94;
  }
  .shutter:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 4px;
  }
  .shutter svg {
    width: 100%;
    height: 100%;
    transform: rotate(-60deg);
  }
  .ring {
    fill: none;
    stroke: var(--accent);
    stroke-width: 6;
    stroke-linecap: round;
    stroke-dasharray: 90 10;
  }
  .core {
    fill: var(--accent);
  }

  .receipt {
    display: grid;
    justify-items: center;
    gap: 8px;
    margin-bottom: 6px;
  }
  .receipt p {
    margin: 0;
  }
</style>
