import { createApp } from 'vue'
import './styles/global.css'
import App from './App.vue'
import { initPaths } from './paths'

initPaths().then(() => createApp(App).mount('#app'))
