import { BaseStationIcon } from "../assets/basestation";
import { motion, AnimatePresence } from "framer-motion"

import VisibilityIcon from "../assets/icons/VisibilityIcon";
import { useState } from "preact/hooks";
import { ChangeBaseStationPowerStatus, IdentitifyBaseStation } from "../../wailsjs/go/main/App";
import { StatusCircleIcon } from "../assets/icons/StatusCircleIcon";
import { PowerStatusIcon } from "../assets/icons/PowerStatusIcon";
import { route } from "preact-router";
import { useTranslation } from "react-i18next";
export function BaseStation({ station, setCurrentBaseStation }) {

    const isAwoke = [0x0B, 0x01, 0x09].includes(station.PowerState);

    const [identitfyDisabled, setIdentitfyhDisabled] = useState(false);    
    const identitify = async () => {
        
        let result = await IdentitifyBaseStation(station.Name);

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
            let result = await ChangeBaseStationPowerStatus(station.Name, "sleep");
            if (result != "ok") return alert(result);

            return setPowerState(0x00);
        }

        //Waking it up
        await ChangeBaseStationPowerStatus(station.Name, "awake");
        setPowerState(0x0B);
    }

    const { t } = useTranslation();
    

    return (<div className="text-white flex flex-row justify-between poppins-medium bg-[#1F1F1F] rounded-sm p-[16px] items-center hover:bg-[#434343] duration-200 cursor-pointer">
        <div className="flex flex-row gap-[16px] items-center" onClick={() => route(`/devices/${station.Name}`)}>
            <BaseStationIcon  />
            <div className="flex flex-col gap-[2px] text-[14px]">
                <span className="flex flex-row gap-[6px] items-center">
                    <span>{station.Name} </span>
                   <StatusCircleIcon class={`data-[awoken="false"]:fill-red-500 fill-green-500 duration-300`}  data-awoken={isAwoke} />

                </span>
                <span className="text-[#C6C6C6]">
                    {t("Channel")} {station.Channel}
                </span>
            </div>
        </div>

        <div className="flex flex-row gap-[8px] [&>*]:flex [&>*]:items-center">
            <AnimatePresence>

            
            {isAwoke ? <motion.div key={"identitfy"} initial={{opacity: 0}} animate={{opacity: 1}} exit={{opacity: 0}}>
                <button  className={"[&>svg]:fill-[#C6C6C6] opacity-75 hover:opacity-100 duration-150 disabled:opacity-25"} onClick={identitify} disabled={identitfyDisabled}>
                    <VisibilityIcon />
                </button>
            </motion.div>
            : null}

            <motion.div key={"awoke"}>
                <button className="opacity-75 hover:opacity-100 duration-150 disabled:opacity-25" onClick={updatePowerState}>
                   <PowerStatusIcon class={`fill-[#C6C6C6]`} />
                </button>
            </motion.div>
            {/* <button key={"open"}>
                <ChevronRightIcon />
            </button> */}
            
            </AnimatePresence>
        </div>
    </div>)
}