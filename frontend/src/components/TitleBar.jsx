import { SettingsIcon, XIcon } from "lucide-preact";
import { IsSteamVRConnected, IsSteamVRConnectivityAvailable, Shutdown, SleepAllBaseStations, WakeUpAllBaseStations } from "../../wailsjs/go/main/App";
import { useEffect, useState } from "preact/hooks";
import { AnimatePresence, motion } from 'framer-motion';



export function TitleBar() {
    
    const [steamVRAvailable, setSteamVRAvailable] = useState(false);
    const [steamVRLaunched, setSteamVRLaunched] = useState(false);
    const [previousState, setPreviousState] = useState(false);

    useEffect(() => {
        (async () => {
            setSteamVRAvailable(await IsSteamVRConnectivityAvailable());
        })()
    }, []);

    useEffect(() => {
        
        let interval;
        
        if (steamVRAvailable) {
            interval = setInterval(async () => {
                let isLaunched = await IsSteamVRConnected();
                setSteamVRLaunched(isLaunched);
            }, 3000);
        }

        return () => clearInterval(interval)
    }, [steamVRAvailable])

    useEffect(() => {
        (async () => {
            if (steamVRLaunched && previousState) {
                setPreviousState(false);
                return await WakeUpAllBaseStations();
            }

            setPreviousState(true);
            return await SleepAllBaseStations();
        })()
    }, [steamVRLaunched])

    return <div className="flex flex-row justify-between pt-[16px] px-[24px] select-none" style={"--wails-draggable:drag"}>
        <div className="flex gap-1 flex-row">
            <div>
                {/* ICON */}
            </div>
            <div className="text-[#888888] poppins-medium text-[14px]/[20px]">
                SteamVR Lighthouse Manager
            </div>
        </div>
        <div className="flex flex-row gap-1 items-center">
            <AnimatePresence>
                {steamVRAvailable && <motion.div initial={{opacity: 0}} animate={{ opacity: 1 }} exit={{ opacity: 0}} ><div className="flex flex-row gap-[4px] text-white poppins-regular text-[14px]">
                    <span className="text-[##C6C6C6]">SteamVR</span>
                    <span className={`data-[active="true"]:text-[#7AFF73] text-[#FF7373] duration-200`} data-active={steamVRLaunched}>{steamVRLaunched ? "Active" : "Inactive"} </span>
                </div></motion.div>}
            </AnimatePresence>
            <div>
                <SettingsIcon color="#888888" size={"18px"}/>
            </div>
            <button onClick={() => Shutdown()}>
                <XIcon color="#888888"/>
            </button>
        </div>
    </div>
}