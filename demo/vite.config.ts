import { UserConfig } from 'vite';

const config: UserConfig = {
  build: {
    target: "esnext",
  },
  server: {
    headers: {
      'Cross-Origin-Opener-Policy': 'same-origin',
      'Cross-Origin-Embedder-Policy': 'require-corp',
      'Cross-Origin-Resource-Policy': 'same-origin'
    }
  }
}

export default config
