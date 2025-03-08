
import { motion } from 'framer-motion'

export function Checkbox({ value, SetValue }) {
    return (<button className={`cursor-pointer w-[42px] h-[24px] rounded-[20px] border-[2px] border-[#888888] bg-[#434343] flex items-center p-[2px] duration-100 data-[active="true"]:bg-[#1D81FF] data-[active="true"]:border-[#9EC9FF]`} data-active={value} onClick={() => SetValue(!value) }>
        <motion.div initial={{ x: value ? 0 : 15 }} animate={{ x: value ? 18 : 0 }} transition={{ type: "spring", bounce: 0, duration: 0.100 }} className={`w-[14px] h-[14px] p-[3px] bg-[#C6C6C6] rounded-[16px] data-[active="true"]:bg-white duration-100`} data-active={value}>
        </motion.div>
    </button>)
}