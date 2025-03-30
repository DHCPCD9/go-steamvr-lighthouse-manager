import { Checkbox } from "./Checkbox";


export function CheckboxOption({ value, setValue, title, description }) {
    return (<div className='flex flex-row justify-between items-center w-full bg-[#1F1F1F] p-[16px] rounded-[6px]'>
        <div>
            <div className='flex flex-col'>
                <span className='text-white text-[14px] poppins-regular'>
                    {title}
                </span>
                <span className='text-white text-[12px] opacity-80 poppins-regular'>
                    {description}
                </span>
            </div>
        </div>
        <div>
            <Checkbox value={value} SetValue={setValue} />
        </div>
    </div>)
}