import { useRouter } from "preact-router"
import { ContainerTitleBar } from "../components/ContainerTitleBar"
import { useState, useEffect } from 'preact/hooks'
import { ChangeBaseStationChannel, GetFoundBaseStations, GetVersion } from "../../wailsjs/go/main/App"
import { smoothResize } from "../utils/windows";
import { ChevronIcon } from "../assets/icons/ChevronIcon"
import { AnimatePresence, motion } from 'framer-motion'

export function BaseStationSettingsPage() {

    const [{ matches },] = useRouter();
    const [lighthouse, setLighthouse] = useState();
    const [channelChangeActive, setChannelChangeActive] = useState(false);
    const [otherBaseStations, setOtherBaseStations] = useState();
    const [channel, setChannel] = useState();



    useEffect(() => {
        (async () => {
            await smoothResize(700, 188, 150);


            let baseStations = await GetFoundBaseStations();
            setLighthouse(baseStations[matches.id]);

            setChannel(baseStations[matches.id].Channel);

        })()
    }, []);

    

    useEffect(() => {
        (async () => {
            let baseStations = await GetFoundBaseStations();
            setOtherBaseStations(Object.values(baseStations));
        })()
    }, [])
    useEffect(() => {
        (async () => {
            if (channelChangeActive) {
                return await smoothResize(700, 236, 150);
            }

            return await smoothResize(700, 188, 150);
        })()
    }, [channelChangeActive]);


    const updateChannel = async (channel) => {
        //Checking if there is any conflicts

        if (otherBaseStations.find(c => c.Channel == channel)) return;

        if (channel < 1 && channel > 16) return alert("???");

        await ChangeBaseStationChannel(lighthouse.Name, channel);

        let baseStations = await GetFoundBaseStations();
        setOtherBaseStations(Object.values(baseStations));
        setChannel(channel);
    }


    return (<div className="flex flex-col gap-2 select-none">
        <div className="text-white py-[12px] px-[24px] text-[24px] poppins-medium flex flex-row gap-[12px] items-center">
            <ContainerTitleBar items={["Devices", matches.id]} />
        </div>
        <div className="text-white poppins-regular px-[24px]">
            <div className="bg-[#1F1F1F] min-h-[70px] py-[12px] px-[8px] rounded-[6px] flex flex-col justify-between items-center">
                <div className="flex flex-row justify-between items-center px-[8px] w-full">
                    <div className="flex flex-col gap-[2px]">
                        <span className="text-[16px]">Channel</span>
                        <span className="text-[14px] opacity-80">
                            Base station channel to operate
                        </span>
                    </div>
                    <div className="flex flex-col items-center">
                        <button className="bg-[#121212] w-[52px] h-[24px] rounded-[6px] pr-[8px] flex flex-row items-center hover:cursor-pointer" onClick={() => setChannelChangeActive(!channelChangeActive)}>
                            <div className="px-[15px] py-[3px] text-[12px]">
                                {channel}
                            </div>
                            <div className={`data-[active="true"]:rotate-180 duration-150`} data-active={channelChangeActive} >
                                <ChevronIcon />
                            </div>
                        </button>
                    </div>
                </div>
                {lighthouse && <AnimatePresence>
                    {channelChangeActive && <motion.div key={"channels"} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.150 }} className="flex flex-row items-center max-w-full py-[6px] gap-[6px]">
                        {[...Array(16).keys()].map((v) => <div
                            className={`px-[12px] py-[8px] rounded-[5px] text-[12px] data-[active="true"]:bg-[#1D81FF] data-[occupied="true"]:bg-[#121212] data-[occupied="true"]:text-[#888888] data-[can-be-selected="true"]:hover:bg-[#434343] duration-200 data-[can-be-selected="true"]:cursor-pointer`}
                            data-can-be-selected={!(lighthouse.Channel == v + 1) && !otherBaseStations.find(c => c.Channel == v + 1 && c.Name != lighthouse.Name)}
                            data-active={channel == v + 1}
                            data-occupied={!!otherBaseStations.find(c => c.Channel == v + 1 && c.Name != lighthouse.Name)}
                            onClick={() => updateChannel(v + 1)}
                        >
                            {v + 1}
                        </div>)}
                    </motion.div>}
                </AnimatePresence> }
            </div>
        </div>
    </div>)
}