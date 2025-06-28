import { useEffect, useState } from "preact/hooks";
import { RenameGroup, UpdateGroupManagedFlags } from "@src/lib/native/index";
import { Link, useRouter } from "preact-router";
import { useContainerTitlebar } from "@src/lib/stores/titlebar.store";
import { InputOption } from "../../components/Input";
import { useTranslation } from "react-i18next";
import { CheckboxOption } from "../../components/CheckboxOption";
import { useGroupedLighthouses } from "@hooks/useGroupedLighthouses";
import { useLighthouseGroup } from "@hooks/useLighthouseGroup";
import { useDebounce } from "@uidotdev/usehooks"
import { smoothResize } from "../../utils/windows";


export function GroupedBaseStationSettings() {


    const [{ matches }, push] = useRouter();


    if (!matches) return push("/");
    const [name, setName] = useState(matches.name!);
    
    const { setItems } = useContainerTitlebar();
    const baseStations = useGroupedLighthouses(name);
    const group = useLighthouseGroup(matches.name);
    const { t } = useTranslation();
    const groupName = useDebounce(name, 1000);
    
    useEffect(() => {

        (async () => {
            await RenameGroup(name, groupName);
            setName(groupName);
            await push(`/groups/${groupName}/settings`, true);
        })()
    }, [groupName]);



    const UpdateManageFlags = async (flag: number) => {

        if ((group.managed_flags & flag) > 0) {
            await UpdateGroupManagedFlags(group.name, group.managed_flags & ~flag)
            return;
        }

        await UpdateGroupManagedFlags(group.name, group.managed_flags | flag)

    }

    const deleteGroup = async (e: Event) => {

    }


    useEffect(() => {
        (async () => {
            if (!group) return push("/");

            setItems([{ text: "Devices", link: "/" }, { text: name, link: `/groups/${name}` }, { text: "Settings", link: `/groups/${group.name}/settings` }]);

            await smoothResize(700, 355);
        })()
    }, [groupName]);

    return (<div className="flex flex-col gap-2 select-none">
        <div className="text-white poppins-regular px-[24px]">
            <InputOption maxLength={16} key={"nickname"} title={t("Group name")} description={t("Display group name.")} value={name} setValue={(e: any) => setName(e)} />
        </div>

        <div className="text-white poppins-regular px-[24px]">
            <CheckboxOption title={"Manage Power (Power on)"} description={"Wake up base stations on SteamVR Exit"} value={group && ((group.managed_flags & 2) > 0)} setValue={() => UpdateManageFlags(2)} />
        </div>

        <div className="text-white poppins-regular px-[24px]">
            <CheckboxOption title={"Manage Power (Power off)"} description={"Power off base stations on SteamVR Exit"} value={group && ((group.managed_flags & 4) > 0)} setValue={() => UpdateManageFlags(4)} />
        </div>
        <div className="px-[24px]">
        <button className="bg-[#1F1F1F] text-[#C6C6C6] poppins-regular w-full py-[4px] text-[14px] rounded-[6px] hover:bg-[#434343] duration-200 cursor-pointer active:bg-[#0D0D0D]!" onClick={deleteGroup}>
            Delete group
        </button>
        </div>
    </div>)
}