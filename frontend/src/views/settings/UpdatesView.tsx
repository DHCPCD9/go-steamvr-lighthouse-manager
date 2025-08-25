import { useEffect, useState } from 'preact/hooks'
import { ForceUpdate, GetConfiguration, IsSteamVRConnectivityAvailable, IsUpdatingSupported, UpdateConfigValue } from '@src/lib/native/index.ts';
import { Trans, useTranslation } from 'react-i18next';
import { motion } from 'framer-motion'
import { useContainerTitlebar } from '@src/lib/stores/titlebar.store.ts';
import { useConfig } from '@src/lib/hooks/useConfig';
import { usePlatform } from '@src/lib/hooks/usePlatform';
import { Link } from 'preact-router';
import { DropdownOption } from '@src/components/Dropdown';
import { ContainerTitleBar } from '@src/components/ContainerTitleBar';

export function SoftwareUpdatesView() {
    const { t } = useTranslation();


    const { setItems, setCallbackOnLast } = useContainerTitlebar();
    const [updateDropdownOpen, setUpdateDropdownOpen] = useState(false);

    const config = useConfig();
    const platform = usePlatform();

    const updateBranch = async (branch: { title: string, value: string }) => {
        await UpdateConfigValue("branch", branch.value);

        await ForceUpdate();
    }

    const checkForUpdates = async () => {
        if (platform.system == "windows" && !platform.flags.includes("NO_UPDATES")) {
            await ForceUpdate();
        }
    }

    useEffect(() => {
        setItems([{ text: "SteamVR LM", link: "/" }, { text: t("Settings"), link: "/settings" }, { text: t("Updates"), link: "/settings/updates" }])
    }, [])

    return (<div className="poppins-semibold text-white py-[12px] px-[24px] select-none">
        <div className='flex flex-col gap-[8px] w-full pt-[8px]'>

            <DropdownOption title={t("Update branch")} description={t("Once you have made that change, you will be asked to update the app.")} setValue={updateBranch} value={{ title: config.branch, value: config.branch }} items={[{ title: "beta", value: "beta" }, { title: "main", value: "main" }]} open={updateDropdownOpen} setOpen={setUpdateDropdownOpen} lockedValues={[]} />
            <div className='flex flex-row justify-between items-center w-full bg-[#1F1F1F] duration-200 cursor-pointer p-[16px] rounded-[6px]'>
                <div>
                    <div className='flex flex-col'>
                        <span className='text-white text-[14px] poppins-regular'>
                            {t("Force update")}
                        </span>
                        <motion.span initial={{ opacity: 0 }} animate={{ opacity: 1 }} className='text-white text-[12px] opacity-80 poppins-regular'>
                            {t("Force update the app.")}
                        </motion.span>
                    </div>
                </div>
                <div>
                    <motion.button initial={{ opacity: 0 }} animate={{ opacity: 1 }} disabled={false} className='text-[12px] py-[8px] px-[32px] disabled:bg-[#2A63AB] disabled:hover:bg-[#2A63AB] disabled:cursor-not-allowed cursor-pointer bg-[#1D81FF] rounded-[6px] duration-100 hover:bg-[#66AAFF] active:bg-[#1D81FF]' onClick={checkForUpdates}>
                        {t("Update")}
                    </motion.button>
                </div>
            </div>

        </div>



    </div>)
}