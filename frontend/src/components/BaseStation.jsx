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
import { ChevronRightIcon, CirclePower, Eye, Power, Settings } from "lucide-preact";
export function BaseStation({ station, onSelect, selected }) {

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
            return;
        }

        //Waking it up
        await ChangeBaseStationPowerStatus(station.id, "awake");
    }

    const { t } = useTranslation();

    const baseStationStatus = useMemo(() => {

        if (station.status == "error") return "error";

        if (station.status != "ready") return "preloaded"

        return isAwoke ? "awoke" : "sleep"
    }, [station.power_state, station.status])


    return (<div className={`text-white flex flex-row justify-between poppins-medium bg-[#1F1F1F] rounded-sm p-[16px] items-center hover:bg-[#434343] data-[selected="true"]:bg-[#434343] duration-200 cursor-pointer active:bg-[#1F1F1F]!`} data-selected={selected}>
        <div className="flex flex-row gap-[16px] items-center" onClick={() => {
            onSelect();
        }}>
            <BaseStationIcon />
            <div className="flex flex-col gap-[2px] text-[14px]">
                <span className="flex flex-row gap-[6px] items-center">
                    <span>{station.name} </span>
                    <StatusCircleIcon class={`data-[status="sleep"]:fill-red-500 data-[status="preloaded"]:fill-blue-500 data-[status="awoke"]:fill-green-500 duration-300`} data-status={baseStationStatus} />

                </span>
                <AnimatePresence mode="wait">
                    {station.status == "ready" && <motion.span className="text-[#C6C6C6]" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                        {t("Channel")} {station.channel}
                    </motion.span>}

                    {station.status != "ready" && <motion.span className="text-[#C6C6C6]" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                        Connecting...
                    </motion.span>}


                </AnimatePresence>

            </div>
        </div>

        <div className="flex flex-row gap-[8px] [&>*]:flex [&>*]:items-center">
            <AnimatePresence>


                {isAwoke && station.status == "ready" ? <motion.div key={"identitfy"} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                    <button className={"opacity-75 hover:opacity-100 duration-150 disabled:opacity-25 cursor-pointer p-1 border-[#C6C6C6] border-none rounded-md"} onClick={identitify} disabled={identitfyDisabled}>
                        <Eye color="#C6C6C6" strokeWidth={2} />
                    </button>
                </motion.div>
                    : null}

                <motion.div key={"awoke"}>
                    {station.status == "ready" && <button className="opacity-75 hover:opacity-100 duration-150 disabled:opacity-25 cursor-pointer p-1 border-[#C6C6C6] border-none rounded-md" onClick={updatePowerState}>
                        <CirclePower color="#C6C6C6" strokeWidth={2} />
                    </button>}
                </motion.div>
                {/* <button key={"open"}>
                <ChevronRightIcon />
            </button> */}
                <button key={"Settings"} className="opacity-75 hover:opacity-100 duration-150 disabled:opacity-25 cursor-pointer p-1 border-[#C6C6C6] border-none rounded-md" onClick={() => route(`/devices/${station.id}`)} >
                    {/* <TitleBarSettingsIcon  class={`fill-[#C6C6C6]`}  /> */}
                    <ChevronRightIcon />

                </button>
            </AnimatePresence>
        </div>
    </div>)
}