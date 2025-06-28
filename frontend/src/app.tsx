//@ts-ignore
import './app.css';
import { BaseStationsList } from './views/BaseStationView';
import { SoftwareSettings } from './views/SoftwareSettings';
import { TitleBar } from './components/TitleBar';
import Router, { useRouter } from 'preact-router'
import { BaseStationSettingsPage } from './views/BaseStationSettings';
import { useContainerTitlebar } from '@src/lib/stores/titlebar.store.ts';
import { ContainerTitleBar } from './components/ContainerTitleBar';
import { motion, AnimatePresence } from 'framer-motion';
import { GroupedBaseStations } from './assets/icons/GroupedBaseStations';
import { GroupedBaseStationView } from './views/groups/GroupedBaseStationView';
import { GroupedBaseStationSettings } from './views/groups/GroupedBaseStationSettingsView';
import { useEffect, useRef } from 'preact/hooks';
import { WebsocketProvider } from '@src/lib/context/websocket.context.tsx';

const AnimatedPath = ({ children, path }) => {
    return (<motion.div key={path} initial={{ opacity: 0, x: -5 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: 5 }}>
        {children}
    </motion.div>)
}
export function App(props) {

    const { items, callbackOnClick, callbackOnLastClick } = useContainerTitlebar();

    return (
        <WebsocketProvider>
            <motion.div>
                <TitleBar />
                <ContainerTitleBar items={items} onLastItemClick={callbackOnLastClick} beforeExit={callbackOnClick} />
                <AnimatePresence mode='wait'>
                    <Router>
                        <AnimatedPath path={"/"}>
                            <BaseStationsList />
                        </AnimatedPath>
                        <AnimatedPath path={"/settings"}>
                            <SoftwareSettings />
                        </AnimatedPath>
                        <AnimatedPath path={"/devices/:id"}>
                            <BaseStationSettingsPage />
                        </AnimatedPath>
                        <AnimatedPath path={"/groups/:name"}>
                            <GroupedBaseStationView />
                        </AnimatedPath>
                        <AnimatedPath path={"/groups/:name/settings"}>
                            <GroupedBaseStationSettings />
                        </AnimatedPath>
                    </Router>
                </AnimatePresence>
            </motion.div>
        </WebsocketProvider>
    )
}
