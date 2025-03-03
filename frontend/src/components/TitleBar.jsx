import { SettingsIcon, XIcon } from "lucide-preact";
import { Shutdown } from "../../wailsjs/go/main/App";



export function TitleBar() {
    return <div className="flex flex-row justify-between pt-[16px] px-[24px] select-none" style={"--wails-draggable:drag"}>
        <div className="flex gap-1 flex-row">
            <div>
                {/* ICON */}
            </div>
            <div className="text-[#888888] poppins-medium text-[14px]/[20px]">
                SteamVR Lighthouse Manager
            </div>
        </div>
        <div className="flex flex-row gap-1 items-center">
            <div>
                <SettingsIcon color="#888888" size={"18px"}/>
            </div>
            <button onClick={() => Shutdown()}>
                <XIcon color="#888888"/>
            </button>
        </div>
    </div>
}