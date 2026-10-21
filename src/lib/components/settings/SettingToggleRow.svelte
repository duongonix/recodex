<script lang="ts">
  import type { Component } from "svelte";
  import { Switch } from "../ui/switch/index.js";

  let {
    id,
    label,
    description,
    icon: Icon,
    value,
    disabled = false,
    disabledReason,
    onChange
  }: {
    id: string;
    label: string;
    description?: string;
    icon: Component;
    value: boolean;
    disabled?: boolean;
    disabledReason?: string;
    onChange: (value: boolean) => Promise<boolean>;
  } = $props();

  let pending = $state(false);

  async function commit(next: boolean) {
    if (disabled || pending || next === value) return;
    pending = true;
    try {
      await onChange(next);
    } finally {
      pending = false;
    }
  }
</script>

<div class="setting-row" class:setting-row-disabled={disabled} title={disabled ? disabledReason : undefined}>
  <Icon aria-hidden="true" class="setting-icon" />
  <div class="setting-copy">
    <span id={`${id}-label`} class="setting-label">{label}</span>
    {#if description || (disabled && disabledReason)}
      <span id={`${id}-hint`} class="setting-hint">{disabled ? disabledReason : description}</span>
    {/if}
  </div>
  <Switch
    checked={value}
    disabled={disabled || pending}
    aria-labelledby={`${id}-label`}
    aria-describedby={disabled && disabledReason ? `${id}-hint` : undefined}
    onCheckedChange={(next) => { void commit(next); }}
  />
</div>
