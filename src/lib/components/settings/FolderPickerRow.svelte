<script lang="ts">
  import Folder from "@lucide/svelte/icons/folder";
  import FolderOpen from "@lucide/svelte/icons/folder-open";

  let {
    id,
    label,
    path,
    disabled = false,
    onChoose
  }: {
    id: string;
    label: string;
    path: string;
    disabled?: boolean;
    onChoose: () => void;
  } = $props();
</script>

<div class="folder-setting-row">
  <Folder aria-hidden="true" class="setting-icon" />
  <div class="folder-setting-content">
    <label class="setting-label" for={id}>{label}</label>
    <div class="folder-field-row">
      <input id={id} class="folder-path" type="text" value={path} readonly aria-label={label} title={path} />
      <button
        type="button"
        class="folder-picker-button"
        aria-label={`Choose ${label.toLowerCase()}`}
        title={`Choose ${label.toLowerCase()}`}
        disabled={disabled}
        onclick={onChoose}
      >
        <FolderOpen aria-hidden="true" class="size-4" />
      </button>
    </div>
  </div>
</div>
