import { useRouter } from "preact-router"
import { useContainerTitlebar } from "@src/lib/stores/titlebar.store";
import { useEffect, useState } from "preact/hooks";
import { BaseStation } from "../../components/BaseStation";
import { AnimatePresence, motion } from 'framer-motion'
import { useGroupedLighthouses } from "@hooks/useGroupedLighthouses";
import { smoothResize } from "../../utils/windows";
import { useLighthouseGroup } from "@src/lib/hooks/useLighthouseGroup";


export function GroupedBaseStationView() {

    const [{ matches }, push] = useRouter();

    if (!matches || !matches.name) return push("/");
    const { setItems } = useContainerTitlebar();
    const baseStations = useGroupedLighthouses(matches.name);
    const group = useLighthouseGroup(matches.name);


    if (!group) return push("/");


    useEffect(() => {
        setItems([{ text: "Devices", link: "/" }, { text: group.name!, link: `/groups/${matches.name}` }]);
        (async () => {
            await smoothResize(700, 445);
        })()
    }, []);
    return (<div className="flex flex-col gap-2 select-none justify-between  px-[24px]" key={"base stations"} style={{ scrollbarWidth: 'none' }}>
        <AnimatePresence>
            <div className="flex flex-col gap-[8px] py-[2px] h-[320px] overflow-y-auto" key={"base stations"} style={{ scrollbarWidth: 'none' }}>

                {baseStations.map((station, index) =>
                    <motion.div key={index} initial={{ opacity: 0, x: -30 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -20 }} transition={{ delay: (index + 1) * 0.150 }}>
                        <BaseStation station={station} key={index + 1} selected={false} />
                    </motion.div>
                )}
            </div>
            <button className="bg-[#1F1F1F] text-[#C6C6C6] poppins-regular w-full py-[4px] text-[14px] rounded-[6px] hover:bg-[#434343] duration-200 cursor-pointer active:bg-[#0D0D0D]!" onClick={() => push(`/groups/${matches.name}/settings`)}>
                Edit group
            </button>
        </AnimatePresence>
    </div>)
}