import { BaseStation } from "./components/BaseStation";
import { useEffect, useState } from 'preact/hooks'
import { motion, AnimatePresence } from 'framer-motion'
import { Loader, Loader2Icon } from "lucide-preact";
import { GetFoundBaseStations, InitBluetooth } from "../wailsjs/go/main/App";
export function BaseStationsList() {

    const [searching, setSearching] = useState(true);
    const [baseStations, setBaseStations] = useState([]);
    const [message, setMessage] = useState();
    const [scanInterval, setScanInterval] = useState();

    useState(() => {
        (async () => {
            if (searching) {

                if (!await InitBluetooth()) {
                    return alert("Unable to init bluetooth.");
                }

                setInterval(async () => {
                    let baseStations = await GetFoundBaseStations();

                    setBaseStations(baseStations);
                }, 300);
                
                
                setTimeout(() => {
                    setSearching(false);
                }, 10000);


            }
        })()
    }, [searching]);
    
    
    return (<div className="flex flex-col gap-2 select-none">
    <div className="text-white py-[12px] px-[24px] text-[24px] poppins-medium">
        Devices
    </div>
    <div className="flex flex-col gap-[8px] px-[24px] py-[2px] max-height-[165px]">
        {/* <BaseStation station={{ Name: "LHB-123456", Channel: 5, PowerState: 2, OldFirmware: false}}/>
        <BaseStation station={{ Name: "LHB-123456", Channel: 5, PowerState: 2, OldFirmware: false}}/>
         */}
         <AnimatePresence>
   
            {/* {message ? <motion.div initial={{opacity: 0, x: -20 }} animate={{opacity: 1, x: 0}}>
                <div className="bg-red-500/60 w-full h-12 ">
                    w
                </div>
            </motion.div> : null} */}
            {baseStations.map((station, index) => 
                <motion.div key={index} initial={{opacity: 0, x: -30}} animate={{ opacity: 1, x: 0}} transition={{ delay: (index + 1) * 0.150 }}>
                    <BaseStation station={station} key={index + 1}/>
                </motion.div>
            )}

                {searching ? <motion.div key={99} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                    <div className="flex flex-row justify-center">
                        <Loader2Icon className={"animate-spin"} color="white"/>
                    </div>
            </motion.div> : null}
            {!baseStations.length && !searching && <motion.div key={1337} initial={{ opacity: 0}} animate={{ opacity: 1}} exit={{ opacity: 0 }} className="poppins-regular text-center text-[14px] text-[#C6C6C6]">
                No base stations seem to be found, maybe other programs are connected to them?    
            </motion.div>}
         </AnimatePresence>
    </div>
    </div>)
}