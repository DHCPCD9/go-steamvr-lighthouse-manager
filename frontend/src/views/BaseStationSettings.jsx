import { useRouter } from "preact-router"
import { ContainerTitleBar } from "../components/ContainerTitleBar"
import { useState, useEffect } from 'preact/hooks'
import { ChangeBaseStationChannel, GetFoundBaseStations, GetVersion } from "../../wailsjs/go/main/App"
import { smoothResize } from "../utils/windows";
import { ChevronIcon } from "../assets/icons/ChevronIcon"
import { AnimatePresence, motion } from 'framer-motion'
import { useTranslation } from "react-i18next";
import { DropdownOption } from "../components/Dropdown";

export function BaseStationSettingsPage() {

    const [{ matches },] = useRouter();
    const [lighthouse, setLighthouse] = useState();
    const [channelChangeActive, setChannelChangeActive] = useState(false);
    const [otherBaseStations, setOtherBaseStations] = useState();
    const [channel, setChannel] = useState();
    const { t, i18n } = useTranslation();


    useEffect(() => {
        (async () => {
            await smoothResize(700, 188, 150);


            let baseStations = await GetFoundBaseStations();
            setLighthouse(baseStations[matches.id]);

            setChannel(baseStations[matches.id].channel);

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
            <ContainerTitleBar items={[t("Devices"), matches.id]} />
        </div>
        <div className="text-white poppins-regular px-[24px]">
            <DropdownOption key={"dropdown"} setValue={updateChannel} lockedValues={otherBaseStations ? otherBaseStations.map(c => c.channel) : []} title={t("Channel")} description={t("Base station channel to operate")} open={channelChangeActive} setOpen={setChannelChangeActive} value={{ title: channel, value: channel }} items={[...Array(16).keys().map(c => {
                return { title: c + 1, value: c + 1 }
            })]} />
        </div>
    </div>)
}