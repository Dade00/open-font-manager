import { createRoot } from 'react-dom/client';

import { App } from './App';
import './styles.css';
import { checkForAppUpdates } from './updater';

const root = document.getElementById('root');
if (!root) throw new Error('App root element is missing');

createRoot(root).render(<App />);
void checkForAppUpdates();
