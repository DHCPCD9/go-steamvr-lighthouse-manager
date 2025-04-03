import { route, useRouter } from "preact-router"
import { ContainerTitleBar } from "../components/ContainerTitleBar"
import { useState, useEffect } from 'preact/hooks'
import { ChangeBaseStationChannel, ForgetBaseStation, GetFoundBaseStations, GetVersion, UpdateBaseStationParam } from "../../wailsjs/go/main/App"
import { smoothResize } from "../utils/windows";
import { ChevronIcon } from "../assets/icons/ChevronIcon"
import { AnimatePresence, motion } from 'framer-motion'
import { useTranslation } from "react-i18next";
import { DropdownOption } from "../components/Dropdown";
import { InputOption } from "../components/Input";
import { Checkbox } from "../components/Checkbox";
import { CheckboxOption } from "../components/CheckboxOption";

export function BaseStationSettingsPage() {

    const [{ matches }] = useRouter();
    const [lighthouse, setLighthouse] = useState();
    const [channelChangeActive, setChannelChangeActive] = useState(false);
    const [otherBaseStations, setOtherBaseStations] = useState();
    const [channel, setChannel] = useState();
    const { t } = useTranslation();
    const [nickname, setNickname] = useState();


    useEffect(() => {
        (async () => {
            await smoothResize(700, 462, 150);


            let baseStations = await GetFoundBaseStations();
            setLighthouse(baseStations[matches.id]);

            setChannel(baseStations[matches.id].channel);

            setNickname(baseStations[matches.id].name)

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
                return await smoothResize(700, 510, 150);
            }

            return await smoothResize(700, 462, 150);
        })()
    }, [channelChangeActive]);


    const updateChannel = async (channel) => {
        //Checking if there is any conflicts

        if (otherBaseStations.find(c => c.Channel == channel)) return;

        if (channel < 1 && channel > 16) return alert("???");

        await ChangeBaseStationChannel(lighthouse.id, channel);

        let baseStations = await GetFoundBaseStations();
        setOtherBaseStations(Object.values(baseStations));
        setChannel(channel);
    }

    const forgetBaseStation = async () => {
        await ForgetBaseStation(lighthouse.id);
        route("/");
    }

    const UpdateManageFlags = async (flag) => {
        if (lighthouse.managed_flags & flag) {
            await UpdateBaseStationParam(lighthouse.id, "managed_flags", lighthouse.managed_flags & ~flag)
            //Removing it
            setLighthouse({ ...lighthouse, managed_flags: lighthouse.managed_flags & ~flag});

            return;
        }

        await UpdateBaseStationParam(lighthouse.id, "managed_flags", lighthouse.managed_flags | flag)
        //Removing it
        setLighthouse({ ...lighthouse, managed_flags: lighthouse.managed_flags | flag});

        
    }

    return (<div className="flex flex-col gap-2 select-none">
        <div className="text-white py-[12px] px-[24px] text-[24px] poppins-medium flex flex-row gap-[12px] items-center">
            <ContainerTitleBar items={[t("Devices"), matches.id]} beforeExit={() => {
            UpdateBaseStationParam(lighthouse.id, "nickname", nickname.trim() ? nickname : lighthouse.id);
                
            }} />
        </div>

        <div className="text-white poppins-regular px-[24px]">
            <InputOption maxLength={16} key={"nickname"} title={t("Nickname")} description={t("Base station nickname to display")} placeholder={lighthouse && lighthouse.id} value={nickname} setValue={setNickname} />
        </div>
        <div className="text-white poppins-regular px-[24px]">
            <DropdownOption key={"dropdown"} setValue={updateChannel} lockedValues={otherBaseStations ? otherBaseStations.map(c => c.channel) : []} title={t("Channel")} description={t("Base station channel to operate")} open={channelChangeActive} setOpen={setChannelChangeActive} value={{ title: channel, value: channel }} items={[...Array(16).keys().map(c => {
                return { title: c + 1, value: c + 1 }
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