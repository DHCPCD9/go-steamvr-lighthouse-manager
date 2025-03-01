import { ArrowUp, BedIcon, BedSingleIcon, PencilIcon } from "lucide-preact"
import BaseStationIcon from "../assets/images/basestation.png"
import { ChangeBaseStationPowerStatus } from "../../wailsjs/go/main/App"
import { toast } from "react-toastify"
import { useState } from "preact/hooks"


export function BaseStation({ station, rescan, setChannel, updateBaseStation }) {

    const [powerState, setPowerState] = useState(station.PowerState);
    const [channel, setBaseStationChannel] = useState(station.Channel);

    console.log(JSON.stringify(station))
    return (<div className="min-h-8 w-full bg-[#1F1F1F] flex flex-row items-center px-2 py-1.5 border-[2px] border-[#323232] rounded-lg justify-between">
        <div className="flex flex-row">
            <img width={48} height={48} src={BaseStationIcon} /> 
            <div className="flex flex-col gap-0.5">
                <div>{station.Name}</div>
                <div className="text-[14px] flex flex-row gap-2">
                    <div>
                        Channel {channel}
                    </div>
                    <div>
                        Running status {powerState}
                    </div>
                </div>
            </div>
        </div>
        <div className="flex flex-row gap-2">
        {powerState != 0 && <button className="p-1.5 bg-[#967EFF] rounded-full hover:bg-[#B5A5FF] duration-200" title="Standing By" onClick={async () => {
                let status = ChangeBaseStationPowerStatus(station.Name, "standingby");

                if (status != "ok") {
                    return toast.error(status);
                }
                setPowerState(1);
                return toast.success("Base station state was updated to standing by.")
            }}>
                <BedSingleIcon size={18}/>
            </button>}
            {powerState == 0 ? <button className="p-1.5 bg-[#1DA91A] rounded-full hover:bg-[#B5A5FF] duration-200" title="Awake base station" onClick={async () => {
                let status = await ChangeBaseStationPowerStatus(station.Name, "awake");


                if (status != "ok") {
                    return toast.error(status);
                }

                setPowerState(1);
                return toast.success("Base station has been awoken.")
            }}> <ArrowUp size={18} /> </button>: <button className="p-1.5 bg-[#443196] rounded-full hover:bg-[#B5A5FF] duration-200" title="Sleep mode" onClick={async () => {
                let status = await ChangeBaseStationPowerStatus(station.Name, "sleep");


                if (status != "ok") {
                    return toast.error(status);
                }
                setPowerState(0);
                return toast.success("Base station has been put in sleep mode.");
            }}>
                <BedIcon size={18}/>
            </button> }
            <button className="p-1.5 bg-[#443196] rounded-full hover:bg-[#B5A5FF] duration-200" title="Edit Channel" onClick={async () => {
                let r = prompt("Enter channel between 1 and 16");

                let parsed = parseInt(r);

                if (isNaN(parsed)) return alert("It doesn't seem to be channel");

                let result = await setChannel(station.Name, parsed);

                console.log(result)
                if (!result.ok) {
                    return alert(result.message);
                }

                setBaseStationChannel(parsed);
                updateBaseStation(station.Name, { Channel: channel, PowerState: powerState})
            }}>
                <PencilIcon size={18}/>
            </button>
            
        </div>
    </div>)
}