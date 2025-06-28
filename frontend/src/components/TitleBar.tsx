import { PowerCircle, SettingsIcon, X, XIcon } from "lucide-preact";
import { ChangeBaseStationPowerStatus, UpdateConfigValue } from "@src/lib/native/index";
import { useContext, useEffect, useState } from "preact/hooks";
import { AnimatePresence, motion } from 'framer-motion';
import { PowerStatusIcon } from "../assets/icons/PowerStatusIcon";
import { TitleBarSettingsIcon } from "../assets/icons/TitleBarSettingsIcon";
import { CloseIcon } from "../assets/icons/CloseIcon";
import { route } from "preact-router";
import { useTranslation } from "react-i18next";
import { useLighthouses } from "@src/lib/hooks/useLighthouses";
import { useConfig } from "@src/lib/hooks/useConfig";
import { useSteamVRStatus } from "@src/lib/hooks/useSteamVRStatus";
import { WebsocketContext } from "@src/lib/context/websocket.context";
import { usePlatform } from "@src/lib/hooks/usePlatform";
import { useLighthouseGroups } from "@src/lib/hooks/useLighthouseGroups";



export function TitleBar() {

    const { websocket, send } = useContext(WebsocketContext);
    const steamVRLaunched = useSteamVRStatus();
    const platform = usePlatform();
    const groups = useLighthouseGroups();
    const [previousSteamVRState, setPreviousSteamVRState] = useState(false);
    const lighthouses = useLighthouses();
    const config = useConfig();

    const { t } = useTranslation();

    const bulkUpdate = async (state: "sleep" | "awake", flags: number = 0) => {
        
        for(const baseStation of lighthouses) {
            if (flags && !((baseStation.managed_flags & flags) > 0)) continue;
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

                for (const value of Object.values(groups)) {
                    if (value.managed_flags & 2) {
                        for (const lighthouseSerial of value.base_stations) {
                            let lighthouse = lighthouses.find(c => c.id == lighthouseSerial);
                            if (lighthouse) {
                                await ChangeBaseStationPowerStatus(lighthouse.id, "awake");
                            }
                        }
                    }
                }
                setPreviousSteamVRState(steamVRLaunched);
                return;
            }

            console.log("Putting in sleep mode")
            await bulkUpdate("sleep", 4);

              for (const value of Object.values(groups)) {
                    if (value.managed_flags & 4) {
                        for (const lighthouseSerial of value.base_stations) {
                            let lighthouse = lighthouses.find(c => c.id == lighthouseSerial);
                            if (lighthouse) {
                                await ChangeBaseStationPowerStatus(lighthouse.id, "sleep");
                            }
                        }
                    }
                }

            setPreviousSteamVRState(steamVRLaunched);
        })()
    }, [steamVRLaunched]);

  
    const toggleAllBaseStations = async () => {
        let status = [...lighthouses.map(c => c.power_state)][0];

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
            //@ts-ignore
            await window.runtime.Hide()

            if (!config.tray_notified) {
                //@ts-ignore
                await window.go.main.App.Notify("SteamVR Lighthosue Manager", t("Window was hidden in the tray."))
                await UpdateConfigValue("tray_notified", true)
            }

            return
        }
        
        //@ts-ignore
        return await window.runtime.Quit()
    }

    return <div className="flex flex-row justify-between pt-[16px] px-[24px] select-none" style="--wails-draggable:drag">
        <div className="flex gap-1 flex-row">
            <div className="text-[#888888] poppins-medium text-[14px]/[20px]">
                SteamVR Lighthouse Manager <span className={"text-[#505050] poppins-medium text-[12px]/[20px]"}>{platform.version}</span>
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

                {!websocket.ready && <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} ><div className="poppins-regular text-sm items-center px-2">
                    <span className={`text-[#FF7373] bg-[#FF7373]/20 rounded-xl border-[#FF7373] border-[0.5px] p-1 text-sm`}>Server connection failed.</span>
                </div></motion.div>}


            </AnimatePresence>
            <button className="opacity-75 hover:opacity-100 duration-150 disabled:opacity-25" onClick={toggleAllBaseStations}>
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