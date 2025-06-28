import { defineConfig } from 'vite'
import preact from '@preact/preset-vite'
import tailwindcss from '@tailwindcss/vite'


// https://vite.dev/config/
export default defineConfig({
  plugins: [preact(), tailwindcss()  ],
  resolve: {
    alias: {
      '@src': "/src",
      '@hooks': "/src/lib/hooks" // path.resolve(__dirname, 'src', "lib", "hooks"),
    },
  }
})
