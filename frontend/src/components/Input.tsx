export function InputOption({ title, description, value, setValue, placeholder, maxLength, onUnfocus }: { title: string, description: string, value: any, setValue?: (value: any) => void, onUnfocus?: (value: any) => void, placeholder?: any, maxLength: number }) {


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
                    {/* <button className="bg-[#121212] min-w-[52px] h-[24px] rounded-[6px] pr-[8px] flex flex-row items-center hover:cursor-pointer" onClick={() => setOpen(!open)}>
                    <div className="px-[15px] py-[3px] text-[12px]">
                        {value.title}
                    </div>
                    <div className={`data-[active="true"]:rotate-180 duration-150`} data-active={open} >
                        <ChevronIcon />
                    </div>
                </button> */}
                    <input maxLength={maxLength} onFocusOut={(e: any) => onUnfocus && onUnfocus(e.target.value)} onChange={(e: any) => setValue && setValue(e.target.value)} value={value} className="w-[260px] h-[24px] bg-[#121212] text-[#888888] rounded-md text-[12px] px-[12px] py-[3px] focus:outline-none poppins-medium" placeholder={placeholder}>

                    </input>
                </div>
            </div>
        </div>
    </div>
}