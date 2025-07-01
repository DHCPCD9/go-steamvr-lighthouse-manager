import { BaseStation } from "../components/BaseStation";
import { useContext, useEffect, useState } from 'preact/hooks'
import { motion, AnimatePresence } from 'framer-motion'
import { Copy, Loader, Loader2Icon, RefreshCcw } from "lucide-preact";
import { AddBaseStationToGroup, CreateGroup, GetConfiguration, InitBluetooth } from "@src/lib/native/index";
import { smoothResize } from "../utils/windows";
import { ContainerTitleBar } from "../components/ContainerTitleBar";
import { useTranslation } from "react-i18next";
import { useContainerTitlebar } from "@src/lib/stores/titlebar.store";
import { useRouter } from "preact-router";
import { deepEqual } from "../utils/arrays";
import { BaseStationGroup } from "../components/BaseStationGroups";
import { useLighthouses } from "@hooks/useLighthouses";
import { useLighthouseGroups } from "@hooks/useLighthouseGroups";
import { useConfig } from "@src/lib/hooks/useConfig";
import { WebsocketContext } from "@src/lib/context/websocket.context";
import { StartScanFor10Seconds } from "../../wailsjs/go/main/App";
export function BaseStationsList() {

    
    const baseStations = useLighthouses();
    const groups = useLighthouseGroups();
    const config = useConfig();
    const { scanning } = useContext(WebsocketContext);

    const [, push] = useRouter();
    const [selectedBaseStations, setSelectedBaseStation] = useState([]);
    const { setItems, setCallbackOnLast } = useContainerTitlebar();

    const { t } = useTranslation();

    useEffect(() => {
        (async () => {
            await InitBluetooth();
            await smoothResize(700, 450);
        })()
    }, []);


    const selectBaseStation = async (id) => {
        if (selectedBaseStations.includes(id)) return setSelectedBaseStation(selectedBaseStations.filter(c => c != id));

        setSelectedBaseStation([...selectedBaseStations, id]);
    }

    const groupBaseStations = async () => {

        let newName = "New Group"
        let i = 0;

        
        for (const group of Object.keys(config.groups)) {
            if (group == newName + (i > 0 ? `(${i + 1})` : "")) {
                i++;
            }

            break
        }


        let group = await CreateGroup(newName, selectedBaseStations);
        setSelectedBaseStation([]);
        return push(`/groups/${group}`);
    }

    useEffect(() => {
        if (selectedBaseStations.length > 0) {
            setCallbackOnLast(() => {
                setSelectedBaseStation([]);
            })
            return setItems([{ text: "Devices", link: "/" }, { text: `${selectedBaseStations.length} Selected`, link: "/" }])
        }

        setItems([{ text: "Devices", link: "/" }])
    }, [])


    return (<div className="flex flex-col gap-2 select-none justify-between">
        <div>
            <AnimatePresence>
                <div className="flex flex-col gap-[8px] px-[24px] py-[2px] h-[335px] overflow-y-auto" key={"base stations"} style={{ scrollbarWidth: 'none' }}>
                {Object.entries(groups).filter(([k,v]) => v).map(([k, v]) => (<BaseStationGroup key={`${k}-group`} id={k} group={v} />))}

                    {baseStations.map((station, index) =>
                        <motion.div key={index} initial={{ opacity: 0, x: -30 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }} transition={{ delay: (index + 1) * 0.150 }}>
                            <BaseStation station={station} key={index + 1} selected={selectedBaseStations.includes(station.id)} onSelect={() => selectBaseStation(station.id)} />
                        </motion.div>
                    )}

                    {<motion.div key={99} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                        <div className="flex flex-row justify-center" onClick={() => StartScanFor10Seconds()}>
                            <RefreshCcw data-disabled={scanning} className={`text-white/80 duration-200 hover:text-white active:text-white/80 data-[disabled="false"]:cursor-pointer data-[disabled="true"]:text-white/40 data-[disabled="true"]:animate-spin`}/>
                        </div>
                    </motion.div>}
                    {!baseStations.length && !scanning && <motion.div key={1337} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="poppins-regular text-center text-[14px] text-[#C6C6C6]">
                        {t("No base stations seem to be found, maybe other programs are connected to them?")}
                    </motion.div>}
                </div>


            
            {selectedBaseStations.length > 0 && baseStations.length > 0 && <motion.div className="px-[24px] z-10" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ ease: "linear"}} >
                <button  className="bg-[#1F1F1F] text-[#C6C6C6] poppins-regular w-full py-[4px] text-[14px] rounded-[6px] hover:bg-[#434343] duration-200 cursor-pointer active:bg-[#0D0D0D]!" onClick={() => groupBaseStations()}>
                    Create group
                </button>
            </motion.div>}
            
            </AnimatePresence>

        </div>
    </div>)
}