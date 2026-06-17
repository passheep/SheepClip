import { createApp } from 'vue';
import App from './App.vue';
import Floating from './Floating.vue';
import './styles.css';

interface TauriWindowMetadata {
  metadata?: {
    currentWindow?: {
      label?: string;
    };
  };
}

function currentWindowLabel() {
  const internals = '__TAURI_INTERNALS__' in window ? (window.__TAURI_INTERNALS__ as TauriWindowMetadata) : null;
  return internals?.metadata?.currentWindow?.label ?? 'main';
}

createApp(currentWindowLabel() === 'floating' ? Floating : App).mount('#app');
