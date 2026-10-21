<script lang="ts">
  import ExternalLink from "@lucide/svelte/icons/external-link";
  import Link from "@lucide/svelte/icons/link";
  import Mail from "@lucide/svelte/icons/mail";
  import UserRound from "@lucide/svelte/icons/user-round";
  import packageInfo from "../../../../package.json";

  const appVersion = packageInfo.version;
</script>

<section class="about-view" aria-labelledby="about-heading">
  <header class="about-heading">
    <p class="about-eyebrow">APPLICATION</p>
    <h2 id="about-heading">About RecoDex</h2>
    <p class="about-subtitle">A simple space to capture what matters.</p>
  </header>

  <section class="app-identity" aria-label="Application information">
    <span class="app-identity-mark" aria-hidden="true">
      <span class="app-identity-dot"></span>
    </span>
    <div class="app-identity-copy">
      <h3>RecoDex</h3>
      <p>Desktop recorder &amp; screenshot</p>
    </div>
    <span class="app-version">v{appVersion}</span>
  </section>

  <section class="author-section" aria-labelledby="author-heading">
    <div class="author-heading">
      <UserRound aria-hidden="true" class="size-4" />
      <h3 id="author-heading">Author</h3>
    </div>

    <p class="author-name">DuongOnix</p>

    <div class="author-links">
      <a
        class="author-link"
        href="https://fb.com/duongonix"
        target="_blank"
        rel="noreferrer"
        aria-label="Open DuongOnix on Facebook"
      >
        <span class="author-link-icon" aria-hidden="true">
          <Link class="size-4" />
        </span>
        <span class="author-link-copy">
          <span class="author-link-label">Facebook</span>
          <span class="author-link-value">fb.com/duongonix</span>
        </span>
        <span class="author-link-arrow" aria-hidden="true"><ExternalLink class="size-3.5" /></span>
      </a>

      <a
        class="author-link"
        href="mailto:duongonixx@gmail.com"
        aria-label="Email DuongOnix at duongonixx@gmail.com"
      >
        <span class="author-link-icon" aria-hidden="true">
          <Mail class="size-4" />
        </span>
        <span class="author-link-copy">
          <span class="author-link-label">Email</span>
          <span class="author-link-value">duongonixx@gmail.com</span>
        </span>
        <span class="author-link-arrow" aria-hidden="true"><ExternalLink class="size-3.5" /></span>
      </a>
    </div>
  </section>
</section>

<style>
  .about-view {
    display: flex;
    min-height: 0;
    flex-direction: column;
    gap: 22px;
    overflow-y: auto;
    padding: 14px 2px 8px;
    scrollbar-width: thin;
    scrollbar-color: var(--border) transparent;
  }

  .about-view::-webkit-scrollbar {
    width: 6px;
  }

  .about-view::-webkit-scrollbar-thumb {
    border-radius: 999px;
    background: var(--border);
  }

  .about-heading {
    display: grid;
    gap: 5px;
  }

  .about-eyebrow {
    margin: 0 0 2px;
    color: var(--accent);
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.12em;
  }

  .about-heading h2 {
    margin: 0;
    color: var(--text);
    font-size: 20px;
    font-weight: 650;
    letter-spacing: -0.035em;
  }

  .about-subtitle {
    margin: 0;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
  }

  .app-identity {
    display: flex;
    min-height: 76px;
    align-items: center;
    gap: 12px;
    padding: 12px;
    border-radius: 12px;
    background: var(--surface);
  }

  .app-identity-mark {
    display: grid;
    width: 42px;
    height: 42px;
    flex: none;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--accent) 54%, transparent);
    border-radius: 13px;
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
  }

  .app-identity-dot {
    width: 18px;
    height: 18px;
    border: 2px solid var(--accent);
    border-radius: 50%;
    box-shadow: inset 0 0 0 4px var(--surface), 0 0 12px color-mix(in srgb, var(--accent) 38%, transparent);
  }

  .app-identity-copy {
    display: grid;
    min-width: 0;
    gap: 3px;
  }

  .app-identity-copy h3 {
    margin: 0;
    color: var(--text);
    font-size: 14px;
    font-weight: 600;
  }

  .app-identity-copy p {
    margin: 0;
    color: var(--muted);
    font-size: 11px;
  }

  .app-version {
    margin-left: auto;
    padding: 5px 8px;
    border-radius: 7px;
    background: var(--surface-hover);
    color: var(--muted);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  .author-section {
    display: grid;
    gap: 12px;
  }

  .author-heading {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--accent);
  }

  .author-heading h3 {
    margin: 0;
    color: var(--text);
    font-size: 13px;
    font-weight: 600;
  }

  .author-name {
    margin: -5px 0 0 24px;
    color: var(--muted);
    font-size: 12px;
  }

  .author-links {
    display: grid;
    gap: 7px;
  }

  .author-link {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 10px;
    padding: 10px;
    border-radius: 10px;
    background: var(--surface);
    color: inherit;
    text-decoration: none;
    transition: background-color 160ms ease, color 160ms ease;
  }

  .author-link:hover {
    background: var(--surface-hover);
  }

  .author-link:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .author-link-icon {
    display: grid;
    width: 32px;
    height: 32px;
    flex: none;
    place-items: center;
    border-radius: 9px;
    background: color-mix(in srgb, var(--accent) 12%, var(--surface));
    color: var(--accent);
  }

  .author-link-copy {
    display: grid;
    min-width: 0;
    gap: 2px;
  }

  .author-link-label {
    color: var(--muted);
    font-size: 10px;
  }

  .author-link-value {
    overflow: hidden;
    color: var(--text);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .author-link-arrow {
    flex: none;
    margin-left: auto;
    color: var(--muted);
    transition: color 160ms ease, transform 160ms ease;
  }

  .author-link:hover .author-link-arrow {
    transform: translate(1px, -1px);
    color: var(--accent);
  }

  @media (prefers-reduced-motion: reduce) {
    .author-link,
    .author-link-arrow {
      transition: none;
    }
  }
</style>
