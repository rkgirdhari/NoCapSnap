<script lang="ts">
  // Pressed onto the photo when a capture is saved.
</script>

<div class="stamp" role="img" aria-label="Saved on this phone">
  <span class="word">Saved</span>
  <span class="sub">on this phone</span>
</div>

<style>
  .stamp {
    display: grid;
    place-items: center;
    gap: 2px;
    padding: 12px 16px 10px;
    border: 3px double var(--accent);
    border-radius: 8px;
    color: var(--accent);
    background: color-mix(in srgb, var(--dark-purple) 74%, transparent);
    transform: rotate(-7deg);
    animation: press 420ms cubic-bezier(0.2, 0.9, 0.25, 1.15) both;
  }
  .word {
    font-family: var(--serif);
    font-size: 26px;
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    line-height: 1;
  }
  .sub {
    font-size: 11px;
    font-weight: 600;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }
  @keyframes press {
    from {
      opacity: 0;
      transform: rotate(-7deg) scale(1.7);
    }
    to {
      opacity: 1;
      transform: rotate(-7deg) scale(1);
    }
  }
</style>
