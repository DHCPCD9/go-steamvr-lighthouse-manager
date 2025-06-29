import type { Dispatch, StateUpdater } from "preact/hooks";
import { ChevronIcon } from "../assets/icons/ChevronIcon";
import { AnimatePresence, motion } from "framer-motion"

export function DropdownOption<T>({ title, description, value, items, lockedValues, open, setOpen, setValue }:
    { title: string, description: string, value: { value: T, title: string | number  }, items: Array<{ value: T, title: string | number  }>, lockedValues: Array<{ value: T, title: string | number }>, open: boolean, setOpen:  Dispatch<StateUpdater<boolean>>, setValue: (data: {value: T, title: string | number }) => void }) {


    return <div className="text-white poppins-regular">
        <div className="bg-[#1F1F1F] min-h-[70px] py-[12px] px-[8px] rounded-[6px] flex flex-col justify-between items-center">
            <div className="flex flex-row justify-between items-center px-[8px] w-full">
                <div className="flex flex-col gap-[2px]">
                    <span className="text-[16px]">{title}</span>
                    <span className="text-[14px] opacity-80">
                        {description}
                    </span>
                </div>
                <div className="flex flex-col items-center">
                    <button className="bg-[#121212] min-w-[52px] h-[24px] rounded-[6px] pr-[8px] flex flex-row items-center hover:cursor-pointer" onClick={() => setOpen(!open)}>
                        <div className="px-[15px] py-[3px] text-[12px]">
                            {value.title}
                        </div>
                        <div className={`data-[active="true"]:rotate-180 duration-150`} data-active={open} >
                            <ChevronIcon />
                        </div>
                    </button>
                </div>
            </div>
            {open && <AnimatePresence>
                {open && <motion.div key={"values"} initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.150 }} className="flex flex-row items-center max-w-full py-[6px] gap-[6px]">
                    {items.map((v) => <div
                        className={`px-[12px] py-[8px] rounded-[5px] text-[12px] data-[active="true"]:bg-[#1D81FF] data-[occupied="true"]:bg-[#121212] data-[occupied="true"]:text-[#888888] data-[can-be-selected="true"]:hover:bg-[#434343] duration-200 data-[can-be-selected="true"]:cursor-pointer`}
                        data-can-be-selected={value.value != v.value && !lockedValues.filter(c => c !== value.value).map(c => c.value).includes(v.value)}
                        data-active={v.value == value.value}
                        data-occupied={lockedValues.filter(c => c !== value.value).map(c => c.value).includes(v.value) && v.value !== value.value}
                        onClick={() => setValue(v)}
                    >
                        {v.title}
                    </div>
                    )}
                </motion.div>}
            </AnimatePresence>}
        </div>
    </div>
}