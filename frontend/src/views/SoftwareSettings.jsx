import { useEffect, useState } from 'preact/hooks'
import { Checkbox } from '../components/Checkbox'
import { ContainerTitleBar } from '../components/ContainerTitleBar';
import { ForceUpdate, GetConfiguration, GetVersion, ToggleSteamVRManagement } from '../../wailsjs/go/main/App';

export function SoftwareSettings() {

    const [managePower, setManagePower] = useState(false);
    const [config, setConfig] = useState();
    const [version, setVersion] = useState();
    const [updateLocked, setUpdateLocked] = useState();
    const [updateText, sestUpdateText] = useState("Check"); // I think there is better way to do it, but it works anyways

    useEffect(() => {
        (async () => {
            await window.runtime.WindowSetSize(700, 278);

            let config = await GetConfiguration();
            setConfig(config);
            setManagePower(config.is_steamvr_managed);
            setVersion(await GetVersion());
        })()
    }, []);

    useEffect(() => {
        (async () => {
            if (!config) return;
            if (managePower == config.is_steamvr_managed) return;
            let newConfig = await ToggleSteamVRManagement();
            setConfig(newConfig);
            setManagePower(newConfig.is_steamvr_managed);
        })()
    }, [managePower, config]);

    const checkForUpdates = async () => {
        setUpdateLocked(true);
        let onlineVersion = await fetch("https://raw.githubusercontent.com/DHCPCD9/go-steamvr-lighthouse-manager/refs/heads/main/VERSION").then(r => r.text()).then(v => v.trim());

        if (version != onlineVersion) {
            await ForceUpdate();
            
            return;
        }

        sestUpdateText("You are on the latest version!");
        setUpdateLocked(false);

        setTimeout(() => {
            sestUpdateText("Check");
        }, 1500);
    } 
    
    return (<div className="poppins-semibold text-white py-[12px] px-[24px] select-none">
        <ContainerTitleBar items={["SteamVR LM", "Settings"]}/>

        <div className='flex flex-col gap-[8px] w-full pt-[8px]'>
            <div className='flex flex-row justify-between items-center w-full bg-[#1F1F1F] p-[16px] rounded-[6px]'>
                <div>
                    <div className='flex flex-col'>
                        <span className='text-white text-[14px] poppins-regular'>
                            Manage Base Station Power
                        </span>
                        <span className='text-white text-[12px] opacity-80 poppins-regular'>
                            Manage Power based on SteamVR launched or not
                        </span>
                    </div>
                </div>
                <div>
                    <Checkbox value={managePower} SetValue={setManagePower}/>
                </div>
            </div>

            <div className='flex flex-row justify-between items-center w-full bg-[#1F1F1F] p-[16px] rounded-[6px]'>
                <div>
                    <div className='flex flex-col'>
                        <span className='text-white text-[14px] poppins-regular'>
                            Check for updates
                        </span>
                        <span className='text-white text-[12px] opacity-80 poppins-regular'>
                            Check for a new version
                        </span>
                    </div>
                </div>
                <div>
                    <button disabled={updateLocked} className='py-[8px] px-[32px] disabled:bg-[#2A63AB] disabled:hover:bg-[#2A63AB] disabled:cursor-not-allowed cursor-pointer bg-[#1D81FF] rounded-[6px] duration-100 hover:bg-[#66AAFF] cursor-pointer' onClick={checkForUpdates}>
                        {updateText}
                    </button>
                </div>
            </div>
        </div>
        <div className='flex flex-row justify-between pt-[2px]'>
            <div className='text-[12px] poppins-regular flex flex-row gap-[12px]'>
                <span>
                    Made by <button className='text-[#1D81FF] hover:text-[#66AAFF] duration-150 cursor-pointer' onClick={() => window.runtime.BrowserOpenURL("https://lisek.cc")} >Alumi</button>
                </span>
                
                <span>
                    Design by <button className='text-[#1D81FF] hover:text-[#66AAFF] duration-150 cursor-pointer' onClick={() => window.runtime.BrowserOpenURL("https://github.com/klonerovsky183")}>Klonerovsky</button>
                </span>
                </div>

            <span className='poppins-regular text-[12px] text-[#C6C6C6]'>
                v{version}
            </span>
        </div>
        
        
    </div>)
}