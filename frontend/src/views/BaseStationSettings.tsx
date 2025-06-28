import { route, useRouter } from "preact-router"
import { ContainerTitleBar } from "../components/ContainerTitleBar"
import { useState, useEffect } from 'preact/hooks'
import { ChangeBaseStationChannel, ForgetBaseStation, UpdateBaseStationParam } from "@src/lib/native/index"
import { smoothResize } from "../utils/windows";
import { ChevronIcon } from "../assets/icons/ChevronIcon"
import { AnimatePresence, motion } from 'framer-motion'
import { useTranslation } from "react-i18next";
import { DropdownOption } from "../components/Dropdown";
import { InputOption } from "../components/Input";
import { Checkbox } from "../components/Checkbox";
import { CheckboxOption } from "../components/CheckboxOption";
import { useContainerTitlebar } from "@src/lib/stores/titlebar.store";
import { useLighthouse } from "@hooks/useLighthouse";
import { useLighthouses } from "@hooks/useLighthouses";

export function BaseStationSettingsPage() {

    const [{ matches }] = useRouter();
    const lighthouse = useLighthouse(matches.id);
    const [channelChangeActive, setChannelChangeActive] = useState(false);
    const otherBaseStations = useLighthouses();
    const { t } = useTranslation();
    const { setItems } = useContainerTitlebar();

    useEffect(() => {
        (async () => {
            if (channelChangeActive) {
                return await smoothResize(700, 510);
            }

            return await smoothResize(700, 435);
        })()
    }, [channelChangeActive]);

    const setNickname = async (e) => {

    }

    const updateChannel = async (channel) => {

        if (otherBaseStations.find(c => c.channel == channel)) return;

        if (channel < 1 && channel > 16) return alert("???");

        await ChangeBaseStationChannel(lighthouse.id, channel);
    }

    const forgetBaseStation = async () => {
        await ForgetBaseStation(lighthouse.id);
        route("/");
    }

    const UpdateManageFlags = async (flag) => {
        if (lighthouse.managed_flags & flag) {
            await UpdateBaseStationParam(lighthouse.id, "managed_flags", lighthouse.managed_flags & ~flag)
            return;
        }

        await UpdateBaseStationParam(lighthouse.id, "managed_flags", lighthouse.managed_flags | flag)
    }

    useEffect(() => {
        setItems([{ text: t("Devices"), link: "/" }, { text: matches.id, link: `/devices/${matches.id}`}]);
    }, []);


    if (!lighthouse) return <></>
    return (<div className="flex flex-col gap-2 select-none">
        <div className="text-white poppins-regular px-[24px]">
            <InputOption maxLength={16} key={"nickname"} title={t("Nickname")} description={t("Base station nickname to display")} placeholder={lighthouse && lighthouse.id} value={lighthouse.name} setValue={setNickname} />
        </div>
        <div className="text-white poppins-regular px-[24px]">
            <DropdownOption key={"dropdown"} setValue={updateChannel} lockedValues={otherBaseStations ? otherBaseStations.map(c => c.channel) : []} title={t("Channel")} description={t("Base station channel to operate")} open={channelChangeActive} setOpen={setChannelChangeActive} value={{ title: lighthouse.channel, value: lighthouse.channel }} items={[...Array(16).fill(null).map((_, index) => {
    return { title: index + 1, value: index + 1 };
})]} />
        </div>

        <div className="text-white poppins-regular px-[24px]">
            <CheckboxOption title={"Manage Power (Wake up)"} description={"Awake this base station on SteamVR Launch"} value={lighthouse && ((lighthouse.managed_flags & 2) > 0)} setValue={() => UpdateManageFlags(2)} />
        </div>

        <div className="text-white poppins-regular px-[24px]">
            <CheckboxOption title={"Manage Power (Power off)"} description={"Power off this base station on SteamVR Exit"} value={lighthouse && ((lighthouse.managed_flags & 4) > 0)} setValue={() => UpdateManageFlags(4)} />
        </div>

        <div className="px-[24px]">
        <button className="bg-[#1F1F1F] text-[#C6C6C6] poppins-regular w-full py-[4px] text-[14px] rounded-[6px] hover:bg-[#434343] duration-200 cursor-pointer active:bg-[#0D0D0D]!" onClick={forgetBaseStation}>
            Forget this base station
        </button>
        </div>
    </div>)
}