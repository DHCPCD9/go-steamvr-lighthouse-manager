import './app.css';
import { BaseStationsList } from './BaseStationView';
import { TitleBar } from './components/TitleBar';
import Router from 'preact-router'
export function App(props) {


  

    return (
       <div>
        <TitleBar />
        <Router>
            <BaseStationsList path="/" />
        </Router>
       </div>
    )
}
