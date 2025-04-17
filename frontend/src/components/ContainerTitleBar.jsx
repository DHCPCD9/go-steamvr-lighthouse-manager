import { motion, AnimatePresence } from 'framer-motion'
import { Link, route } from 'preact-router'

export function ContainerTitleBar({ items, beforeExit, onLastItemClick }) {

    const transitionParams = {  initial: { opacity: 0, x: -3 }, animate: { opacity: 1, x: 0 }, exit: { opacity: 0, x: -3 }, transition: { delay: 0.1 } };
    if (!items.length) {
        return (<></>)
    }//flex flex-row text-[24px] text-white py-[6px] px-[24px] text-[24px] poppins-medium flex flex-row items-center gap-[6px]
    return (<div className='flex flex-row text-[24px] text-white poppins-medium px-[24px] items-center gap-[6px] h-[36px]'>
        <AnimatePresence mode='popLayout'>
            {/* <motion.div key={`-1--${items[0].text}`} initial={{ opacity: 0, x: -5 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0 }}>
                <Link className='text-[#C6C6C6] cursor-pointer text-nowrap items-center' onClick={(e) => {
                    e.preventDefault();
                    beforeExit && beforeExit();
                    route("/");
                }}>
                    {items[0].text}
                </Link>
            </motion.div>
            
            {items.slice(1).map((c, i) => (<div key={i} className='flex flex-row pl-[6px] gap-[6px] items-center'>
                <motion.span key={`${i}--seprator`} className='text-[#C6C6C6]' initial={{ opacity: 0, x: -3 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -3 }}>
                    &gt;
                </motion.span>
                <motion.div key={`${i}--${c.text}`} initial={{ opacity: 0, x: -3 }} animate={{ opacity: 1, x: 0 }} exit={{ opacity: 0, x: -3 }} transition={{ delay: 0.1 }}>
                    <Link href={c.link} className={`cursor-pointer text-nowrap data-[active="true"]:text-white text-[#C6C6C6]`} data-active={i == (items.length - 2)} onClick={() => {
                        if (items.at(-1) == c) {
                            onLastItemClick();
                        }
                    }}>
                        {c.text}
                    </Link>
                </motion.div></div>)
            )} */}
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