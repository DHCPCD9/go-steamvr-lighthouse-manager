import { ChevronRightIcon, EyeIcon } from "lucide-preact";
import { BaseStationIcon } from "../assets/basestation";

import VisibilityIcon from "../assets/icons/VisibilityIcon";
export function BaseStation({ station }) {


    return (<div className="text-white flex flex-row justify-between poppins-medium bg-[#1F1F1F] rounded-sm p-[16px] items-center">
        <div className="flex flex-row gap-[16px] items-center">
            <BaseStationIcon  />
            <div className="flex flex-col gap-[2px] text-[14px]">
                <span>
                    {station.Name}
                </span>
                <span className="text-[#C6C6C6]">
                    Channel {station.Channel}
                </span>
            </div>
        </div>
        <div className="flex flex-row gap-[8px]">
            <button>
                <VisibilityIcon />
            </button>
            <button>
                <ChevronRightIcon />
            </button>
        </div>
    </div>)
}