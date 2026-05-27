import { defineConfig } from 'vite';
import { qwikVite } from '@builder.io/qwik/optimizer';
import { qwikCity } from '@builder.io/qwik-city/vite';

export default defineConfig(() => {
  return {
    // GitHub Pages project site lives at /todork/
    base: '/todork/',
    plugins: [qwikCity(), qwikVite()],
    preview: {
      headers: { 'Cache-Control': 'public, max-age=600' },
    },
  };
});
