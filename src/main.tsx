import React, {lazy,Suspense} from 'react';
import { createRoot } from 'react-dom/client';
const Panel=lazy(()=>import('./components/Panel').then(m=>({default:m.Panel})));
const PetWindow=lazy(()=>import('./components/PetWindow').then(m=>({default:m.PetWindow})));
import './styles.css';
const kind=new URLSearchParams(location.search).get('window');
const pet=kind==='duck';
document.body.classList.toggle('pet-body',pet);
createRoot(document.getElementById('root')!).render(<React.StrictMode><Suspense fallback={null}>{pet?<PetWindow/>:<Panel/>}</Suspense></React.StrictMode>);
