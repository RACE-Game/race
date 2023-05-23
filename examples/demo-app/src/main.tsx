import ReactDOM from 'react-dom/client'
import App from './App'
import './index.css'
import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import Raffle from './Raffle';
import DrawCard from './DrawCard';
// import Chat from './Chat';

const router = createBrowserRouter([
  {
    path: "/",
    element: <App />,
    children: [
      // {
      //   path: "chat/:addr",
      //   element: <Chat />
      // },
      {
        path: "raffle/:addr",
        element: <Raffle />,
      },
      {
        path: 'draw-card/:addr',
        element: <DrawCard />,
      }
    ]
  }
]);

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <RouterProvider router={router} />
)
