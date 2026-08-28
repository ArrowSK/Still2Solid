<script lang="ts">
  import BackgroundAdvisor from './BackgroundAdvisor.svelte';

  export let imageUrl = '';
  export let backgroundRemoval = true;
  export let disabled = false;
</script>

<BackgroundAdvisor {imageUrl} bind:enabled={backgroundRemoval} {disabled} />
