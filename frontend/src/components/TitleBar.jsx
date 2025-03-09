import { SettingsIcon, XIcon } from "lucide-preact";
import { ChangeBaseStationPowerStatus, GetConfiguration, GetFoundBaseStations, IsSteamVRConnected, IsSteamVRConnectivityAvailable, Shutdown, SleepAllBaseStations, WakeUpAllBaseStations } from "../../wailsjs/go/main/App";
import { useEffect, useState } from "preact/hooks";
import { AnimatePresence, motion } from 'framer-motion';
import { PowerStatusIcon } from "../assets/icons/PowerStatusIcon";
import { TitleBarSettingsIcon } from "../assets/icons/TitleBarSettingsIcon";
import { CloseIcon } from "../assets/icons/CloseIcon";
import { route } from "preact-router";
import { useTranslation } from "react-i18next";



export function TitleBar() {

    const [steamVRAvailable, setSteamVRAvailable] = useState(false);
    const [steamVRLaunched, setSteamVRLaunched] = useState(false);
    const [previousState, setPreviousState] = useState(false);
    const [steamVRManagementEnabled, setSteamVRManagementEnabled] = useState(true);
    const { t } = useTranslation();

    const bulkUpdate = async (state) => {
        
        for(const baseSation of Object.values(await GetFoundBaseStations())) {
            await ChangeBaseStationPowerStatus(baseSation.Name, state);
        }
    }

    useEffect(() => {
        (async () => {
            setSteamVRAvailable(await IsSteamVRConnectivityAvailable());
            setSteamVRManagementEnabled(GetConfiguration().then(c => c.is_steamvr_managed));
        })()
    }, []);

    useEffect(() => {

        let interval;

        if (steamVRAvailable) {
            interval = setInterval(async () => {
                setSteamVRManagementEnabled(await GetConfiguration().then(c => c.is_steamvr_managed));
                if (!steamVRManagementEnabled) return;
                let isLaunched = await IsSteamVRConnected();
                setSteamVRLaunched(isLaunched);

            }, 3000);
        }

        return () => clearInterval(interval)
    }, [steamVRAvailable])

    useEffect(() => {
        (async () => {
            if (!steamVRManagementEnabled) return;
            if (steamVRLaunched && !previousState) {
                 setPreviousState(true);
                 return await bulkUpdate("awake");
            }

            setPreviousState(false);
            await bulkUpdate("sleep");

        })()
    }, [steamVRLaunched]);

  
    const toggleAllBaseStations = async () => {
        let status = [...new Set(Object.values(await go.main.App.GetFoundBaseStations()).map(c => c.PowerState))][0];

        if (!status) {
            console.log("Waking up everything");
            setPreviousState(false);
            return await bulkUpdate("awake");
        }

        console.log("Putting all base station in sleep mode");
        setPreviousState(true);
        await bulkUpdate("sleep");
    }

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
                {steamVRAvailable && steamVRManagementEnabled && <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} ><div className="flex flex-row gap-[4px] text-white poppins-regular text-[14px] items-center px-2">
                    <span className="text-[##C6C6C6]">SteamVR</span>
                    <span className={`data-[active="true"]:text-[#7AFF73] text-[#FF7373] duration-200`} data-active={steamVRLaunched}>{steamVRLaunched ? t("Active") : t("Inactive")} </span>
                </div></motion.div>}


            </AnimatePresence>
            <button className="opacity-75 hover:opacity-100 duration-150 disabled:opacity-25" onClick={toggleAllBaseStations}>
                <PowerStatusIcon width={14} height={14} class={`fill-[#C6C6C6]`} />
            </button>
            <button onClick={(c) => route("/settings", true)}>
                <TitleBarSettingsIcon width={16} height={16} fill="#888888" className={`hover:fill-[#1D81FF] duration-200`} />
            </button>
            <button onClick={() => Shutdown()}>
                <CloseIcon  width={12} height={12} fill="#888888" className={`hover:fill-[#1D81FF] duration-200`} />
            </button>
        </div>
    </div>
}