import { useEffect, useState } from 'preact/hooks'
import { route } from 'preact-router'
import { AnimatePresence, motion } from 'framer-motion'
import { Checkbox } from './Checkbox'

export function SoftwareSettings() {

    const [maangePower, setManagePower] = useState(false);
    useEffect(() => {
        (async () => {
            await window.runtime.WindowSetSize(700, 278);
        })()
    }, [])
    return (<div className="poppins-semibold text-white py-[12px] px-[24px] select-none">
        <div className="flex flex-row gap-[12px] text-[24px]">
            <motion.div initial={{ opacity: 0, x: -5 }} animate={{ opacity: 1, x: 0 }} ><button className='text-[#C6C6C6] cursor-pointer' onClick={() => route("/")}>
                SteamVR LM
            </button>
            </motion.div>
            <motion.div initial={{ opacity: 0, x: -10 }} animate={{ opacity: 1, x: 0 }} transition={{ delay: 0.1 }}>
                <span className='text-[#C6C6C6]'>
                    >
                </span>
            </motion.div>
            <motion.div initial={{ opacity: 0, x: -15 }} animate={{ opacity: 1, x: 0 }} transition={{ delay: 0.2 }}>
            <span className='text-white'>
                Settings
            </span>
            </motion.div>
        </div>

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
                    <Checkbox value={maangePower} SetValue={setManagePower}/>
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
                    <button className='py-[8px] px-[32px] bg-[#1D81FF] rounded-[6px] duration-100 hover:bg-[#66AAFF] cursor-pointer'>
                        Check
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
                v0
            </span>
        </div>
        
        
    </div>)
}