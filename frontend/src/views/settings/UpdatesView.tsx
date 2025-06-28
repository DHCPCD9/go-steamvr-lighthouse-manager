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

export function SoftwareSettings() {
    const { t } = useTranslation();

    const { setItems, setCallbackOnLast } = useContainerTitlebar();
    const config = useConfig();

    const updateBranch = (branch: string) => {

    }
    
    useEffect(() => {
        setItems([{ text: "SteamVR LM", link: "/" }, { text: t("Settings"), link: "/settings" }])
    }, [])

    return (<div className="poppins-semibold text-white py-[12px] px-[24px] select-none">
        <div className='flex flex-col gap-[8px] w-full pt-[8px]'>

            <DropdownOption title={t("Updates branch")} description={t("Branch")} setValue={i18n.changeLanguage} value={{ title: languageNames[i18n.language], value: i18n.language }} items={languages.filter(c => c != "cimode").map(c => {
                return { title: languageNames[c] ?? c, value: c }
            })} open={dropdownOpen} setOpen={setDropdownOpen} lockedValues={[]} />

            <div className='flex flex-row justify-between items-center w-full bg-[#1F1F1F] p-[16px] rounded-[6px]'>
                <div>
                    <div className='flex flex-col'>
                        <span className='text-white text-[14px] poppins-regular'>
                            {t("Manage Base Station Power")}
                        </span>
                        <motion.span initial={{ opacity: 0 }} animate={{ opacity: 1 }} className='text-white text-[12px] opacity-80 poppins-regular'>
                            {steamVRAvailable && config.is_steamvr_installed ? t("Manage Power based on SteamVR launched or not") : t("SteamVR not found or this platform is not supported")}
                        </motion.span>
                    </div>
                </div>
                <div>
                    <Checkbox disabled={!steamVRAvailable || !config?.is_steamvr_installed} value={config?.is_steamvr_managed} SetValue={togglePowerManagement} />
                </div>
            </div>

            <div className='flex flex-row justify-between items-center w-full bg-[#1F1F1F] p-[16px] rounded-[6px]'>
                <div>
                    <div className='flex flex-col'>
                        <span className='text-white text-[14px] poppins-regular'>
                            {t("Allow tray")}
                        </span>
                        <span className='text-white text-[12px] opacity-80 poppins-regular'>
                            {t("Run the application in the tray when SteamVR is active or when the main window is closed.")}
                        </span>
                    </div>
                </div>
                <div>
                    <Checkbox value={config?.allow_tray ?? false} SetValue={toggleAllowTray} />
                </div>
            </div>

            {/* @ts-ignore */}
            <Link href={"/settings/updates"} className='flex flex-row justify-between items-center w-full bg-[#1F1F1F] hover:bg-[#434343] active:bg-[#1F1F1F] duration-200 cursor-pointer p-[16px] rounded-[6px]'>
                <div>
                    <div className='flex flex-col'>
                        <span className='text-white text-[14px] poppins-regular'>
                            {t("Updates")}
                        </span>
                        <motion.span initial={{ opacity: 0 }} animate={{ opacity: 1 }} className='text-white text-[12px] opacity-80 poppins-regular'>
                            {t("Force update or set a update branch")}
                        </motion.span>
                    </div>
                </div>
            </Link>
           
        </div>
        


    </div>)
}