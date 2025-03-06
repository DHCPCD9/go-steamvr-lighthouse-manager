import './app.css';
import { BaseStationsList } from './BaseStationView';
import { SoftwareSettings } from './components/SoftwareSettings';
import { TitleBar } from './components/TitleBar';
import Router from 'preact-router'
export function App(props) {

    return (
       <div>
        <TitleBar />
        <Router>
            <BaseStationsList path="/" />
            <SoftwareSettings path="/settings" />
        </Router>
       </div>
    )
}
