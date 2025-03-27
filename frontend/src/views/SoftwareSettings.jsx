import { useEffect, useState } from 'preact/hooks'
import { Checkbox } from '../components/Checkbox'
import { ContainerTitleBar } from '../components/ContainerTitleBar';
import { ForceUpdate, GetConfiguration, GetVersion, IsSteamVRConnectivityAvailable, IsUpdatingSupported, ToggleSteamVRManagement, ToggleTray, UpdateConfigValue } from '../../wailsjs/go/main/App';
import { Trans, useTranslation } from 'react-i18next';
import { DropdownOption } from '../components/Dropdown';
import { smoothResize } from '../utils/windows';
import { motion } from 'framer-motion'

export function SoftwareSettings() {

    //TODO: Reduce hooks amount
    const [config, setConfig] = useState();
    const [isUpdatingSupported, setIsUpdatingSupported] = useState(false);
    const [steamVRAvailable, setsteamVRAvailable] = useState(false);
    const [version, setVersion] = useState();
    const [updateLocked, setUpdateLocked] = useState();
    const { t, i18n } = useTranslation();
    const [updateText, setUpdateText] = useState(t("Check")); // I think there is better way to do it, but it works anyways
    const [dropdownOpen, setDropdownOpen] = useState(false);

    const updateConfig = async () => {
        setConfig(await GetConfiguration());
    }

    useEffect(() => {
        (async () => {
            await smoothResize(700, 435);

            setConfig(await GetConfiguration());
            setVersion(await GetVersion());
            setIsUpdatingSupported(await IsUpdatingSupported());
            setsteamVRAvailable(await IsSteamVRConnectivityAvailable());
        })()
    }, []);

    useEffect(() => {
        (async () => {
            if (dropdownOpen) {
                return await smoothResize(700, 475)
            }

            await smoothResize(700, 435);
        })()
    }, [dropdownOpen])

    const checkForUpdates = async () => {

        if (!isUpdatingSupported) return window.runtime.BrowserOpenURL("https://github.com/DHCPCD9/go-steamvr-lighthouse-manager/releases/latest");
        setUpdateLocked(true);
        let onlineVersion = await fetch("https://raw.githubusercontent.com/DHCPCD9/go-steamvr-lighthouse-manager/refs/heads/main/VERSION").then(r => r.text()).then(v => v.trim());

        if (version != onlineVersion) {
            await ForceUpdate();
            
            return;
        }

        setUpdateText(t("You are on the latest version!"));
        setUpdateLocked(false);

        setTimeout(() => {
            setUpdateText(t("Check"));
        }, 1500);
    } 

    const toggleAllowTray = async () => {
        await UpdateConfigValue("allow_tray", !config.allow_tray);
        await updateConfig();
    }

    const togglePowerManagement = async () => {
        await UpdateConfigValue("is_steamvr_managed", !config.is_steamvr_managed);
        await updateConfig();
    }

    useEffect(() => {

        if (!isUpdatingSupported) {
            return setUpdateText(t("Get new build from Github"));
        }
        setUpdateText(t("Check"));
    }, [i18n.language, isUpdatingSupported]);
    
    const languageNames = {
        en: "English",
        ru: "Русский",
        zh_cn: "简体中文",
        zh_cht: "繁体中文",
        de: "Deutsch",
        kr: "한국어",
        jp: "日本語",
        ua: "Українська"
    }

    const languages = Object.keys(i18n.store.data);

    return (<div className="poppins-semibold text-white py-[12px] px-[24px] select-none">
        <ContainerTitleBar items={["SteamVR LM", t("Settings")]}/>

        <div className='flex flex-col gap-[8px] w-full pt-[8px]'>

            <DropdownOption title={t("Language")} description={t("Language of the interface")} setValue={i18n.changeLanguage} value={{ title: languageNames[i18n.language], value: i18n.language }} items={languages.filter(c => c != "cimode").map(c => {
                return { title: languageNames[c]??c, value: c }
            })} open={dropdownOpen} setOpen={setDropdownOpen} lockedValues={[]}/>
            
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
                    <Checkbox disabled={!steamVRAvailable || !config?.is_steamvr_installed} value={config?.is_steamvr_managed} SetValue={togglePowerManagement}/>
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
                    <Checkbox value={config?.allow_tray??false} SetValue={toggleAllowTray}/>
                </div>
            </div>

            <div className='flex flex-row justify-between items-center w-full bg-[#1F1F1F] p-[16px] rounded-[6px]'>
                <div>
                    <div className='flex flex-col'>
                        <span className='text-white text-[14px] poppins-regular'>
                            {t("Check for updates")}
                        </span>
                        <motion.span initial={{ opacity: 0 }} animate={{ opacity: 1 }} className='text-white text-[12px] opacity-80 poppins-regular'>
                            {isUpdatingSupported ? t("Check for a new version") : t("Updating for portable builds or this platform is not supported.")}
                        </motion.span>
                    </div>
                </div>
                <div>
                    <motion.button initial={{ opacity: 0 }} animate={{ opacity: 1 }} disabled={updateLocked} className='text-[12px] py-[8px] px-[32px] disabled:bg-[#2A63AB] disabled:hover:bg-[#2A63AB] disabled:cursor-not-allowed cursor-pointer bg-[#1D81FF] rounded-[6px] duration-100 hover:bg-[#66AAFF] cursor-pointer' onClick={checkForUpdates}>
                        {updateText}
                    </motion.button>
                </div>
            </div>
        </div>
        <div className='flex flex-row justify-between pt-[2px]'>
            <div className='text-[12px] poppins-regular flex flex-row gap-[12px]'>
                <span>
                    <Trans i18nKey={"Made By"}>
                        Made by <button className='text-[#1D81FF] hover:text-[#66AAFF] duration-150 cursor-pointer' onClick={() => window.runtime.BrowserOpenURL("https://lisek.cc")}>Alumi</button>
                    </Trans>
                </span>
                
                <span>
                    <Trans i18nKey={"Design By"}>
                        Design by <button className='text-[#1D81FF] hover:text-[#66AAFF] duration-150 cursor-pointer' onClick={() => window.runtime.BrowserOpenURL("https://github.com/klonerovsky183")}>Klonerovsky</button>
                    </Trans>
                </span>

                <span>
                    <button className='text-[#1D81FF] hover:text-[#66AAFF] duration-150 cursor-pointer' onClick={() => window.runtime.BrowserOpenURL("https://github.com/DHCPCD9/go-steamvr-lighthouse-manager")}>{t("Source code")}</button>
                </span>
                </div>

            <span className='poppins-regular text-[12px] text-[#C6C6C6]'>
                v{version}
            </span>
        </div>
        
        
    </div>)
}