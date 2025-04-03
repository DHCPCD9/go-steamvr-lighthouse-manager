import { BaseStationIcon } from "../assets/basestation";
import { motion, AnimatePresence } from "framer-motion"

import VisibilityIcon from "../assets/icons/VisibilityIcon";
import { useMemo, useState } from "preact/hooks";
import { ChangeBaseStationPowerStatus, IdentitifyBaseStation } from "../../wailsjs/go/main/App";
import { StatusCircleIcon } from "../assets/icons/StatusCircleIcon";
import { PowerStatusIcon } from "../assets/icons/PowerStatusIcon";
import { route } from "preact-router";
import { useTranslation } from "react-i18next";
import { TitleBarSettingsIcon } from "../assets/icons/TitleBarSettingsIcon";
export function BaseStation({ station, refreshBaseStations, onSelect, selected }) {

    const isAwoke = [0x0B, 0x01, 0x09].includes(station.power_state);

    const [identitfyDisabled, setIdentitfyhDisabled] = useState(false);    
    const identitify = async () => {
        
        let result = await IdentitifyBaseStation(station.id);

        if (result != "ok") return alert(result);

        console.log("Identitfy packet sent");
        setIdentitfyhDisabled(true);
        setTimeout(() => {
            setIdentitfyhDisabled(false);
        }, 20000)
    }

    const updatePowerState = async () => {
        if (isAwoke) {
            //Sleeping of
            let result = await ChangeBaseStationPowerStatus(station.id, "sleep");
            if (result != "ok") return alert(result);

            setPowerState(0x00);

            return await refreshBaseStations();
        }

        //Waking it up
        await ChangeBaseStationPowerStatus(station.id, "awake");
        setPowerState(0x0B);
        await refreshBaseStations();
    }

    const { t } = useTranslation();

    const baseStationStatus = useMemo(() => {
        
        if (station.status == "error") return "error";

        if (station.status != "ready") return "preloaded"

        return isAwoke ? "awoke" : "sleep"
    }, [station.power_state, station.status])
    

    return (<div className={`text-white flex flex-row justify-between poppins-medium bg-[#1F1F1F] rounded-sm p-[16px] items-center hover:bg-[#434343] data-[selected="true"]:bg-[#434343] duration-200 cursor-pointer`} data-selected={selected}>
        <div className="flex flex-row gap-[16px] items-center" onClick={() => {
            onSelect();
        }}>
            <BaseStationIcon  />
            <div className="flex flex-col gap-[2px] text-[14px]">
                <span className="flex flex-row gap-[6px] items-center">
                    <span>{station.name} </span>
                   <StatusCircleIcon class={`data-[status="sleep"]:fill-red-500 data-[status="preloaded"]:fill-blue-500 data-[status="awoke"]:fill-green-500 duration-300`}  data-status={baseStationStatus} />

                </span>
                <span className="text-[#C6C6C6]">
                {station.version == 2 && <>{t("Channel")} {station.channel}</>} {station.version == 1 && <>lighthouse v{station.version} </>}
                </span>
            </div>
        </div>

        <div className="flex flex-row gap-[8px] [&>*]:flex [&>*]:items-center">
            <AnimatePresence>

            
            {isAwoke && station.status == "ready" ? <motion.div key={"identitfy"} initial={{opacity: 0}} animate={{opacity: 1}} exit={{opacity: 0}}>
                <button  className={"[&>svg]:fill-[#C6C6C6] opacity-75 hover:opacity-100 duration-150 disabled:opacity-25 scale-120 cursor-pointer"} onClick={identitify} disabled={identitfyDisabled}>
                    <VisibilityIcon />
                </button>
            </motion.div>
            : null}

            <motion.div key={"awoke"}>
                {station.status == "ready" && <button className="opacity-75 hover:opacity-100 duration-150 disabled:opacity-25 scale-120 cursor-pointer" onClick={updatePowerState}>
                   <PowerStatusIcon class={`fill-[#C6C6C6]`} />
                </button> }
            </motion.div>
            {/* <button key={"open"}>
                <ChevronRightIcon />
            </button> */}
            <button key={"Settings"} className="opacity-75 hover:opacity-100 duration-150 disabled:opacity-25 scale-140 cursor-pointer" onClick={() => route(`/devices/${station.id}`)} >
                <TitleBarSettingsIcon  class={`fill-[#C6C6C6]`}  />
            </button>
            </AnimatePresence>
        </div>
    </div>)
}