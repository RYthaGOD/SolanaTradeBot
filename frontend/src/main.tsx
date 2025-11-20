import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App-PredictionOnly'
import { WalletContextProvider } from './contexts/WalletContextProvider'
import './styles/futuristic.css'
import './styles/enhanced.css'
import './styles/components.css'

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <WalletContextProvider>
      <App />
    </WalletContextProvider>
  </React.StrictMode>,
)
