import { motion } from 'framer-motion'
import { route } from 'preact-router'

export function ContainerTitleBar({ items, beforeExit }) {
    return (<div className='flex flex-row gap-[12px] text-[24px]'>
        
        <motion.div initial={{ opacity: 0, x: -5 }} animate={{ opacity: 1, x: 0 }} >
            <button className='text-[#C6C6C6] cursor-pointer' onClick={() => {
                beforeExit && beforeExit();
                route("/");
            }}>
                {items[0]}
            </button>
        </motion.div>
        {items.filter(c => c).length > 1 && <motion.div initial={{ opacity: 0, x: -10 }} animate={{ opacity: 1, x: 0 }} transition={{ delay: 0.1 }}>
            <span className='text-[#C6C6C6]'>
                &gt;
            </span>
        </motion.div>}
        {items.slice(1).map((c, i) => <motion.div key={i} initial={{ opacity: 0, x: -15 }} animate={{ opacity: 1, x: 0 }} transition={{ delay: 0.2 }}>
            <span className='text-white'>
                {c}
            </span>
        </motion.div>)}
        </div>)
}