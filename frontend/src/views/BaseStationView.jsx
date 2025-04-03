import { BaseStation } from "../components/BaseStation";
import { useEffect, useState } from 'preact/hooks'
import { motion, AnimatePresence } from 'framer-motion'
import { Loader, Loader2Icon } from "lucide-preact";
import { GetFoundBaseStations, InitBluetooth } from "../../wailsjs/go/main/App";
import { smoothResize } from "../utils/windows";
import { ContainerTitleBar } from "../components/ContainerTitleBar";
import { Trans, useTranslation } from "react-i18next";
export function BaseStationsList() {

    const [searching, setSearching] = useState(true);
    const [baseStations, setBaseStations] = useState([]);
    const [activeBaseStation, setActiveBaseStation] = useState();
    const [selectedBaseStations, setSelectedBaseStation] = useState([]);

    const { t } = useTranslation();

    useEffect(() => {
        let interval;

        (async () => {

            if (!await InitBluetooth()) {
                return alert("Unable to init bluetooth.");
            }

            interval = setInterval(async () => {
                await refreshBaseStations();
            }, 300);
        })()

        return () => clearInterval(interval);
    }, [searching]);

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
            await smoothResize(700, 445, 150);
        })()
    }, []);

    const refreshBaseStations = async () => {
        let baseStations = await GetFoundBaseStations();

        setBaseStations(Object.values(baseStations));
    }

    const selectBaseStation = async (id) => {
        // console.log({selectedBaseStations, s: selectedBaseStations.includes(id)})
        // if (selectedBaseStations.includes(id)) return setSelectedBaseStation(selectedBaseStations.filter(c => c != id));

        // setSelectedBaseStation([...selectedBaseStations, id]);
    }

    return (<div className="flex flex-col gap-2 select-none justify-between">
        <div className="text-white py-[6px] px-[24px] text-[24px] poppins-medium flex flex-row gap-[12px] items-center">
            <ContainerTitleBar items={[t("Devices"), activeBaseStation ? activeBaseStation : null]} />
        </div>
        <div>
            <AnimatePresence>

                <div className="flex flex-col gap-[8px] px-[24px] py-[2px] max-h-[330px]">

                    {baseStations.map((station, index) =>
                        <motion.div key={index} initial={{ opacity: 0, x: -30 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }} transition={{ delay: (index + 1) * 0.150 }}>
                            <BaseStation station={station} key={index + 1} refreshBaseStations={refreshBaseStations} setCurrentBaseStation={setActiveBaseStation} selected={selectedBaseStations.includes(station.id)} onSelect={() => selectBaseStation(station.id)} />
                        </motion.div>
                    )}

                    {searching && !baseStations.length ? <motion.div key={99} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                        <div className="flex flex-row justify-center">
                            <Loader2Icon className={"animate-spin"} color="white" />
                        </div>
                    </motion.div> : null}

                    {/* {baseStations.length > 0 && baseStations.length < 4 && <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ delay: ((baseStations.length + 1) * 0.150)}} className="flex justify-center text-white/50 poppins-reglar">
                        <Trans i18nKey={"Found base stations"}  count={baseStations.length}>
                            Found {{ baseStations: baseStations.length }} Lighthouse(s)
                        </Trans>
                    </motion.div>} */}
                    {!baseStations.length && !searching && <motion.div key={1337} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="poppins-regular text-center text-[14px] text-[#C6C6C6]">
                        {t("No base stations seem to be found, maybe other programs are connected to them?")}
                    </motion.div>}
                </div>


            {/* {selectedBaseStations.length > 0 && <motion.div className="px-[24px] z-10" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0}}>
                <button  className="bg-[#1F1F1F] text-[#C6C6C6] poppins-regular w-full py-[4px] text-[14px] rounded-[6px] hover:bg-[#434343] duration-200 cursor-pointer active:bg-[#0D0D0D]!">
                    Create group
                </button>
            </motion.div>} */}
            </AnimatePresence>

        </div>
    </div>)
}