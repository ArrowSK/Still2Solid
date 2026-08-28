<script lang="ts">
  import BackgroundAdvisor from './BackgroundAdvisor.svelte';
  export let imageUrl = '';
  export let enabled = true;
  export let disabled = false;
</script>

<BackgroundAdvisor {imageUrl} bind:enabled {disabled} />
