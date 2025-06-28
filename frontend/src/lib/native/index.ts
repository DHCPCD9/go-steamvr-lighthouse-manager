import { useContext } from "preact/hooks";
import { useConfig } from "../hooks/useConfig";
import { usePlatform } from "../hooks/usePlatform";
import { WebsocketContext, type WebsocketContextType } from "../context/websocket.context";

import * as native from "../../../wailsjs/go/main/App"
type status = "ok" | string

export const ChangeBaseStationPowerStatus = async (send: WebsocketContextType['send'] ,id: string, mode: "sleep" | "awake") => {
    await native.ChangeBaseStationPowerStatus(id, mode);
}

export const RenameGroup = async (old: string, newName: string) => {
    return native.RenameGroup(old, newName);
}

export const UpdateGroupManagedFlags = async (group: string, flags: number) => {
    return native.UpdateGroupManagedFlags(group, flags);
}

export const IdentitifyBaseStation = async (id: string): Promise<status> => {
    return native.IdentitifyBaseStation(id);
}

export const ForceUpdate = async (): Promise<void> => {
    return native.ForceUpdate();
}

export const GetConfiguration = async (): Promise<any> => {
    return {};
}


export const IsSteamVRConnectivityAvailable = async (): Promise<boolean> => {
    const config = useConfig();
    return config.is_steamvr_installed;
}

export const IsUpdatingSupported = async (): Promise<boolean> => {
    const platform = usePlatform();
    return platform.system == "windows";
}

export const UpdateConfigValue = async (key: string, value: any): Promise<void> => {
    return native.UpdateConfigValue(key, value);
}

export const AddBaseStationToGroup = async (stationId: string, groupId: string): Promise<status> => {
    return native.AddBaseStationToGroup(stationId, groupId);
}

export const CreateGroup = async (name: string): Promise<string> => {
    return native.CreateGroup(name);
}

export const InitBluetooth = async (): Promise<status> => {
    return "ok";
}

export const ChangeBaseStationChannel = async (id: string, channel: number): Promise<status> => {
    return native.ChangeBaseStationChannel(id, channel);
}

export const ForgetBaseStation = async (id: string): Promise<void> => {
    return native.ForgetBaseStation(id);
}

export const UpdateBaseStationParam = async (id: string, param: string, value: any): Promise<void> => {
    return native.UpdateBaseStationParam(id, param, value);
}