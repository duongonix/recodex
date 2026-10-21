<script lang="ts">
  import type { Component } from "svelte";
  import {
    Content as SelectContent,
    Item as SelectItem,
    Root as SelectRoot,
    Trigger as SelectTrigger,
    Value as SelectValue
  } from "../ui/select/index.js";

  type Choice = { value: string; label: string; disabled?: boolean; title?: string };

  let {
    id,
    label,
    icon: Icon,
    value,
    choices,
    placeholder,
    disabled = false,
    onChange
  }: {
    id: string;
    label: string;
    icon: Component;
    value: string;
    choices: Choice[];
    placeholder: string;
    disabled?: boolean;
    onChange: (value: string) => Promise<boolean>;
  } = $props();

  let pending = $state(false);

  async function commit(next: string | undefined) {
    if (disabled || pending || !next || next === value) return;
    pending = true;
    try {
      await onChange(next);
    } finally {
      pending = false;
    }
  }
</script>

<div class="setting-row setting-select-row">
  <Icon aria-hidden="true" class="setting-icon" />
  <label id={`${id}-label`} class="setting-label" for={id}>{label}</label>
  <SelectRoot type="single" items={choices} value={value} onValueChange={(next) => { void commit(next); }}>
    <SelectTrigger id={id} class="setting-select" aria-labelledby={`${id}-label`} disabled={disabled || pending}>
      <SelectValue {placeholder} />
    </SelectTrigger>
    <SelectContent>
      {#each choices as choice (choice.value)}
        <SelectItem value={choice.value} label={choice.label} disabled={choice.disabled} title={choice.title} />
      {/each}
    </SelectContent>
  </SelectRoot>
</div>
