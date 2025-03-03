import { ChevronRightIcon, EyeIcon } from "lucide-preact";
import { BaseStationIcon } from "../assets/basestation";
import { motion, AnimatePresence } from "framer-motion"

import VisibilityIcon from "../assets/icons/VisibilityIcon";
import { useState } from "preact/hooks";
import { ChangeBaseStationPowerStatus, IdentitifyBaseStation } from "../../wailsjs/go/main/App";
import { PowerIcon } from "lucide-preact";
export function BaseStation({ station }) {

    const [powerState, setPowerState] = useState(station.PowerState);
    const isAwoke = [0x0B, 0x01, 0x09].includes(powerState);

    console.log(powerState)
    const [identitfyDisabled, setIdentitfyhDisabled] = useState(false);
    const [powerStateLocked, setPowerStateLocked] = useState(false);
    
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

    

    console.log(isAwoke)
    return (<div className="text-white flex flex-row justify-between poppins-medium bg-[#1F1F1F] rounded-sm p-[16px] items-center">
        <div className="flex flex-row gap-[16px] items-center">
            <BaseStationIcon  />
            <div className="flex flex-col gap-[2px] text-[14px]">
                <span>
                    {station.Name}
                </span>
                <span className="text-[#C6C6C6]">
                    Channel {station.Channel}
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
                <button className="opacity-75 hover:opacity-100 duration-150 disabled:opacity-25" disabled={powerStateLocked} onClick={updatePowerState}>
                   <PowerIcon class={`data-[awoken="true"]:text-red-500 text-green-500 duration-300`} size={16}  data-awoken={isAwoke} />
                </button>
            </motion.div>
            <button key={"open"}>
                <ChevronRightIcon />
            </button>
            
            </AnimatePresence>
        </div>
    </div>)
}