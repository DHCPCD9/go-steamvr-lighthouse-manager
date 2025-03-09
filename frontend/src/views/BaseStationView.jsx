import { BaseStation } from "../components/BaseStation";
import { useEffect, useState } from 'preact/hooks'
import { motion, AnimatePresence } from 'framer-motion'
import { Loader, Loader2Icon } from "lucide-preact";
import { GetFoundBaseStations, InitBluetooth } from "../../wailsjs/go/main/App";
import { smoothResize } from "../utils/windows";
import { ContainerTitleBar } from "../components/ContainerTitleBar";
import { useTranslation } from "react-i18next";
export function BaseStationsList() {

    const [searching, setSearching] = useState(true);
    const [baseStations, setBaseStations] = useState([]);
    const [activeBaseStation, setActiveBaseStation] = useState();

    const { t, i18n } = useTranslation();

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
            await smoothResize(700, 445, 150);
        })()
    }, []);

    return (<div className="flex flex-col gap-2 select-none">
        <div className="text-white py-[6px] px-[24px] text-[24px] poppins-medium flex flex-row gap-[12px] items-center">
            <ContainerTitleBar items={[t("Devices"), activeBaseStation ? activeBaseStation : null]} />
        </div>
        <div>
            <AnimatePresence>

                <div className="flex flex-col gap-[8px] px-[24px] py-[2px] max-h-[330px]">

                    {baseStations.map((station, index) =>
                        <motion.div key={index} initial={{ opacity: 0, x: -30 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }} transition={{ delay: (index + 1) * 0.150 }}>
                            <BaseStation station={station} key={index + 1} setCurrentBaseStation={setActiveBaseStation} />
                        </motion.div>
                    )}

                    {searching && !baseStations.length ? <motion.div key={99} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }}>
                        <div className="flex flex-row justify-center">
                            <Loader2Icon className={"animate-spin"} color="white" />
                        </div>
                    </motion.div> : null}
                    {!baseStations.length && !searching && <motion.div key={1337} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} className="poppins-regular text-center text-[14px] text-[#C6C6C6]">
                        {t("No base stations seem to be found, maybe other programs are connected to them?")}
                    </motion.div>}
                </div>

            </AnimatePresence>

        </div>
    </div>)
}