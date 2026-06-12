<script lang="ts">
  import 'img-comparison-slider';
  import 'img-comparison-slider/dist/styles.css';

  interface Props {
    beforeUrl: string;
    afterUrl: string;
    beforeLabel?: string;
    afterLabel?: string;
  }

  const {
    beforeUrl,
    afterUrl,
    beforeLabel = 'Original',
    afterLabel = 'Comprimida',
  }: Props = $props();
</script>

<img-comparison-slider class="compare-slider">
  <img slot="first" src={beforeUrl} alt={beforeLabel} />
  <img slot="second" src={afterUrl} alt={afterLabel} />
</img-comparison-slider>

<style>
  .compare-slider {
    width: 100%;
    border-radius: 6px;
    overflow: hidden;
  }

  :global(img-comparison-slider img) {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }
</style>
