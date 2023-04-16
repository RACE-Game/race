import React from 'react'
import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'
import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import Raffle from './Raffle';
import Chat from './Chat';
import * as solanaWeb3 from '@solana/web3.js';
import * as SPL from '@solana/spl-token';
import * as Metaplex from '@metaplex-foundation/js';

declare global {
    interface Window {
        solanaWeb3: any;
        SPL: any;
        Metaplex: any;
    }
}
window.solanaWeb3 = solanaWeb3;
window.SPL = SPL;
window.Metaplex = Metaplex;

const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      {
        path: "chat/:addr",
        element: <Chat />
      },
      {
        path: "raffle/:addr",
        element: <Raffle />,
      }
    ]
  }
]);

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <RouterProvider router={router} />
)
