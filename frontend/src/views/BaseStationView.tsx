import { BaseStation } from "../components/BaseStation";
import { useContext, useEffect, useState } from 'preact/hooks'
import { motion, AnimatePresence } from 'framer-motion'
import { Copy, Loader, Loader2Icon, PencilIcon, PencilOffIcon, PlusIcon, RefreshCcw } from "lucide-preact";
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
    const [editModeEnabled, setEditModeEnabled] = useState(false);
    

    const { t } = useTranslation();

    useEffect(() => {
        (async () => {
            await InitBluetooth();
            await smoothResize(700, 460);
        })()
    }, []);


    const selectBaseStation = async (id) => {
        if (selectedBaseStations.includes(id)) return setSelectedBaseStation(selectedBaseStations.filter(c => c != id));

        setSelectedBaseStation([...selectedBaseStations, id]);
    }

    const groupBaseStations = async () => {

        if (!selectedBaseStations.length) return;
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

    const toggleEditMode = async () => {
        setEditModeEnabled(!editModeEnabled);
        setSelectedBaseStation([]);
    }

    useEffect(() => {
        if (selectedBaseStations.length > 0) {
            setCallbackOnLast(() => {
                setSelectedBaseStation([]);
            })
            return setItems([{ text: "Devices", link: "/" }, { text: `${selectedBaseStations.length} Selected`, link: "/" }])
        }

        setItems([{ text: "Devices", link: "/" }])
    }, []);


    return (<div className="flex flex-col gap-2 select-none justify-between">
        <div>
            <AnimatePresence>
                <div className="flex flex-col gap-[8px] px-[24px] py-[2px] h-[335px] overflow-y-auto" key={"base stations"} style={{ scrollbarWidth: 'none' }}>
                    {Object.entries(groups).filter(([k, v]) => v).map(([k, v]) => (<BaseStationGroup key={`${k}-group`} id={k} group={v} />))}

                    {baseStations.map((station, index) =>
                        <motion.div key={index} initial={{ opacity: 0, x: -30 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }} transition={{ delay: (index + 1) * 0.150 }}>
                            <BaseStation station={station} key={index + 1} selected={selectedBaseStations.includes(station.id)} onSelect={() => editModeEnabled && selectBaseStation(station.id)} editMode={editModeEnabled} />
                        </motion.div>
                    )}


                    {!baseStations.length && !scanning && <motion.div key={1337} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="poppins-regular text-center text-[14px] text-[#C6C6C6]">
                        {t("No base stations seem to be found, maybe other programs are connected to them?")}
                    </motion.div>}
                </div>


                <div class={"flex flex-row justify-end px-[24px] pb-[12px] gap-2"}>
                    <AnimatePresence>
                        {!editModeEnabled && <motion.div key={99} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                            <div className="flex flex-row justify-center clickable rounded-md" onClick={() => StartScanFor10Seconds()}>
                                <RefreshCcw size={24} data-disabled={scanning} className={`text-white/80 duration-200 hover:text-white active:text-white/80 data-[disabled="false"]:cursor-pointer data-[disabled="true"]:text-white/40 data-[disabled="true"]:animate-spin`} />
                            </div>
                        </motion.div>}

                        {toggleEditMode && <div className={`flex flex-row justify-center clickable rounded-md data-[disabled="true"]:opacity-50 data-[disabled="true"]:cursor-not-allowed`} data-disabled={selectedBaseStations.length < 1} onClick={() => groupBaseStations()}>
                            <PlusIcon size={24} data-disabled={scanning} className={`text-white/80 duration-200 hover:text-white active:text-white/80 data-[disabled="false"]:cursor-pointer data-[disabled="true"]:text-white/40 data-[disabled="true"]:animate-spin`} />
                        </div>}

                        <div  data-active={editModeEnabled} className={`flex flex-row justify-center clickable rounded-md data-[active="true"]:bg-[#1D81FF]`} onClick={() => toggleEditMode()}>
                            <AnimatePresence mode="popLayout">

                                {!editModeEnabled ?
                                    <motion.div key={"edit"} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0, rotate: -180 }} transition={{ type: "tween", easings: ["linear"]}} >
                                        <PencilIcon size={24} data-active={editModeEnabled} className={`text-white/80 duration-200 hover:text-white active:text-white/80 data-[disabled="false"]:cursor-pointer data-[disabled="true"]:text-white/40 data-[disabled="true"]:animate-spin`} /> 
                                    </motion.div> :
                                     <motion.div key={"close"} initial={{ opacity: 0, rotate: -180 }} animate={{ opacity: 1, rotate: -365 }} exit={{ opacity: 0 }} transition={{ type: "tween", easings: ["linear"]}}>
                                        <PencilOffIcon size={24} className={`text-white/80 duration-200 hover:text-white active:text-white/80 data-[disabled="false"]:cursor-pointer data-[disabled="true"]:text-white/40 data-[disabled="true"]:animate-spin`} /> 
                                    </motion.div>}

                            </AnimatePresence>
                        </div>


                        {/* {editModeEnabled && <motion.div className="px-[24px] z-10" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ ease: "linear" }} >
                        <button className="bg-[#1F1F1F] text-[#C6C6C6] poppins-regular w-full py-[4px] text-[14px] rounded-[6px] hover:bg-[#434343] duration-200 cursor-pointer active:bg-[#0D0D0D]! disabled:opacity-50 disabled:cursor-not-allowed" disabled={selectBaseStation.length < 1} onClick={() => groupBaseStations()}>
                            Create group
                        </button>
                    </motion.div>} */}
                    </AnimatePresence>
                </div>





            </AnimatePresence>

        </div>
    </div>)
}