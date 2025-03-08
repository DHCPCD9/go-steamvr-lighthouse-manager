import './app.css';
import { BaseStationsList } from './views/BaseStationView';
import { SoftwareSettings } from './views/SoftwareSettings';
import { TitleBar } from './components/TitleBar';
import Router from 'preact-router'
import { BaseStationSettingsPage } from './views/BaseStationSettings';
export function App(props) {

    return (
       <div>
        <TitleBar />
        <Router>
            <BaseStationsList path="/" />
            <SoftwareSettings path="/settings" />
            <BaseStationSettingsPage path="/devices/:id" />
        </Router>
       </div>
    )
}
