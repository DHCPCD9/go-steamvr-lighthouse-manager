import { BaseStation } from "./components/BaseStation";


export function BaseStationsList() {
    return (<div className="flex flex-col gap-2 select-none">
    <div className="text-white py-[12px] px-[24px] text-[24px] poppins-medium">
        Devices
    </div>
    <div className="flex flex-col gap-[8px] px-[24px] py-[2px] max-height-[165px]">
        <BaseStation station={{ Name: "LHB-123456", Channel: 5, PowerState: 2, OldFirmware: false}}/>
        <BaseStation station={{ Name: "LHB-123456", Channel: 5, PowerState: 2, OldFirmware: false}}/>

    </div>
    </div>)
}