import { motion, AnimatePresence } from 'framer-motion'
import { Link, route } from 'preact-router'

export function ContainerTitleBar({ items, beforeExit, onLastItemClick }) {

    const transitionParams = {  initial: { opacity: 0, x: -3 }, animate: { opacity: 1, x: 0 }, exit: { opacity: 0, x: -3 }, transition: { delay: 0.1 } };
    if (!items.length) {
        return (<></>)
    }//flex flex-row text-[24px] text-white py-[6px] px-[24px] text-[24px] poppins-medium flex flex-row items-center gap-[6px]
    return (<div className='flex flex-row text-[24px] text-white poppins-medium px-[24px] items-center gap-[6px] h-[36px]'>
        <AnimatePresence mode='popLayout'>
            {items.map((c, i) => <motion.div key={i} className='flex flex-row items-center gap-[6px] justify-center'>
                {i > 0 && <motion.div className='flex flex-row items-center text-[#C6C6C6]' {...transitionParams}>
                    &gt;
                </motion.div>}
                <motion.div {...transitionParams}>
                <Link href={c.link} className={`text-nowrap data-[active="true"]:text-white text-[#C6C6C6]`} data-active={i == (items.length - 1)}>
                        {c.text}
                    </Link>
                </motion.div>
            </motion.div>)}
        </AnimatePresence>
    </div>)
}