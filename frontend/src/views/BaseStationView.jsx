import { BaseStation } from "../components/BaseStation";
import { useEffect, useState } from 'preact/hooks'
import { motion, AnimatePresence } from 'framer-motion'
import { Loader, Loader2Icon } from "lucide-preact";
import { GetFoundBaseStations, InitBluetooth } from "../../wailsjs/go/main/App";
import { smoothResize } from "../utils/windows";
import { ContainerTitleBar } from "../components/ContainerTitleBar";
export function BaseStationsList() {

    const [searching, setSearching] = useState(true);
    const [baseStations, setBaseStations] = useState([]);
    const [message, setMessage] = useState();
    const [scanInterval, setScanInterval] = useState();
    const [activeBaseStation, setActiveBaseStation] = useState();

    useEffect(() => {
        let interval;

        (async () => {

            if (!await InitBluetooth()) {
                return alert("Unable to init bluetooth.");
            }

            interval = setInterval(async () => {
                let baseStations = await GetFoundBaseStations();
                
                setBaseStations(Object.values(baseStations));
            }, 300);
        })()
        
        return () => clearInterval(interval);
    }, [searching])
    useEffect(() => {
        (async () => {
            if (searching) {
                setTimeout(() => {
                    setSearching(false);
                }, 10000);
            }
        })()

    }, [searching]);


    useEffect(() => {
        (async () => {
            await smoothResize(700, 400, 150);
        })()
    }, []);

    return (<div className="flex flex-col gap-2 select-none">
        <div className="text-white py-[12px] px-[24px] text-[24px] poppins-medium flex flex-row gap-[12px] items-center">
            <ContainerTitleBar items={["Devices", activeBaseStation ? activeBaseStation : null]}/>
        </div>
        <div>
                  {activeBaseStation && <AnimatePresence key={"base station"}>
                    <motion.div key={"settings"} initial={{ x: -20, opacity: 0}} animate={{ x: 0, opacity: 1 }} exit={{ x: -20, opacity: 0 }}>

                        <BaseStationSettings station={baseStations.find(c => c.Name == activeBaseStation)}/>
                    </motion.div>

</AnimatePresence>}
            <AnimatePresence>
                
                    {!activeBaseStation && <motion.div className="flex flex-col gap-[8px] px-[24px] py-[2px] max-height-[165px]" initial={{ opacity: 1 }} exit={{ x: -20, opacity: 0}}>

                        {baseStations.map((station, index) =>
                            <motion.div key={index} initial={{ opacity: 0, x: -30 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }} transition={{ delay: (index + 1) * 0.150 }}>
                                <BaseStation station={station} key={index + 1} setCurrentBaseStation={setActiveBaseStation} />
                            </motion.div>
                        )}

                        {searching ? <motion.div key={99} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                            <div className="flex flex-row justify-center">
                                <Loader2Icon className={"animate-spin"} color="white" />
                            </div>
                        </motion.div> : null}
                        {!baseStations.length && !searching && <motion.div key={1337} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="poppins-regular text-center text-[14px] text-[#C6C6C6]">
                            No base stations seem to be found, maybe other programs are connected to them?
                        </motion.div>}
                    </motion.div>}
                    
                </AnimatePresence>
                
        </div>
    </div>)
}