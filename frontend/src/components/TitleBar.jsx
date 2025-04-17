import { PowerCircle, SettingsIcon, X, XIcon } from "lucide-preact";
import { ChangeBaseStationPowerStatus, IsSteamVRConnected, IsSteamVRConnectivityAvailable, Shutdown, SleepAllBaseStations, UpdateConfigValue, WakeUpAllBaseStations } from "../../wailsjs/go/main/App";
import { useEffect, useState } from "preact/hooks";
import { AnimatePresence, motion } from 'framer-motion';
import { PowerStatusIcon } from "../assets/icons/PowerStatusIcon";
import { TitleBarSettingsIcon } from "../assets/icons/TitleBarSettingsIcon";
import { CloseIcon } from "../assets/icons/CloseIcon";
import { route } from "preact-router";
import { useTranslation } from "react-i18next";
import { useLighthouses } from "../../lib/hooks/useLighthouses";
import { useConfig } from "../../lib/hooks/useConfig";
import { useSteamVRStatus } from "../../lib/hooks/useSteamVRStatus";



export function TitleBar() {

    const steamVRLaunched = useSteamVRStatus();
    const [previousSteamVRState, setPreviousSteamVRState] = useState(false);
    const lighthouses = useLighthouses();
    const config = useConfig();

    const { t } = useTranslation();

    const bulkUpdate = async (state, flags) => {
        
        for(const baseStation of lighthouses) {
            if (!((baseStation.managed_flags & flags) > 0)) continue;
            await ChangeBaseStationPowerStatus(baseStation.id, state);
        }
    }

    useEffect(() => {
        (async () => {
            if (!config) return;
            if (!config.is_steamvr_managed) return;

            if (steamVRLaunched && !previousSteamVRState) {
                console.log("Waking up")
                await bulkUpdate("awake", 2);
                setPreviousSteamVRState(steamVRLaunched);
                return;
            }

            console.log("Putting in sleep mode")
            await bulkUpdate("sleep", 4);
            setPreviousSteamVRState(steamVRLaunched);
        })()
    }, [steamVRLaunched]);

  
    const toggleAllBaseStations = async () => {
        let status = [...lighthouses.map(c => c.PowerState)][0];

        if (!status) {
            console.log("Waking up everything");
            setPreviousSteamVRState(false);
            return await bulkUpdate("awake");
        }

        console.log("Putting all base station in sleep mode");
        setPreviousSteamVRState(true);
        await bulkUpdate("sleep");
    }

    const Quit = async () => {
        if (config.allow_tray) {
            await window.runtime.Hide()

            if (!config.tray_notified) {
                await window.go.main.App.Notify("SteamVR Lighthosue Manager", t("Window was hidden in the tray."))
                await UpdateConfigValue("tray_notified", true)
            }

            return
        }
        
        return await window.runtime.Quit();
    }

    return <div className="flex flex-row justify-between pt-[16px] px-[24px] select-none" style={"--wails-draggable:drag"}>
        <div className="flex gap-1 flex-row">
            <div className="text-[#888888] poppins-medium text-[14px]/[20px]">
                SteamVR Lighthouse Manager
            </div>
        </div>
        <div>

        </div>
        <div className="flex flex-row gap-1 items-center" style={"--wails-draggable:no-drag"}>
            <AnimatePresence>
                {config.is_steamvr_installed && config && config.is_steamvr_managed && <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} ><div className="flex flex-row gap-[4px] text-white poppins-regular text-[14px] items-center px-2">
                    <span className="text-[##C6C6C6]">SteamVR</span>
                    <span className={`data-[active="true"]:text-[#7AFF73] text-[#FF7373] duration-200`} data-active={steamVRLaunched}>{steamVRLaunched ? t("Active") : t("Inactive")} </span>
                </div></motion.div>}


            </AnimatePresence>
            <button className="opacity-75 hover:opacity-100 duration-150 disabled:opacity-25" onClick={toggleAllBaseStations}>
                {/* <PowerStatusIcon width={14} height={14} class={`fill-[#C6C6C6]`} /> */}
                <PowerCircle color="#C6C6C6"/>
            </button>
            <button onClick={(c) => route("/settings", true)}>
                {/* <TitleBarSettingsIcon width={16} height={16} fill="#888888" className={`hover:fill-[#1D81FF] duration-200`} /> */}
                <SettingsIcon color="#888888" />
            </button>
            <button onClick={() => Quit()}>
                {/* <CloseIcon  width={12} height={12} fill="#888888" className={`hover:fill-[#1D81FF] duration-200`} /> */}
                <X color="#888888" />
            </button>
        </div>
    </div>
}