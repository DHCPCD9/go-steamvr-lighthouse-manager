

export interface LighthouseStation {
  id: string;
  power_state: number;
  status: "error" | "preload" | "sleep" | "ready";
  channel: number;
  name: string;
  managed_flags: number;
}

export interface LighthouseGroup {
  name: string;
  base_stations: string[];
  managed_flags: number;
}


export interface Configuration {
  allow_tray: boolean
  groups: Groups
  is_steamvr_installed: boolean
  is_steamvr_managed: boolean
  known_base_stations: {
    [name: string]: LighthouseStation
  }
  tray_notified: boolean
}


export interface Groups { }

export interface WebsocketPayloadBase {
  event: string;
  data: any;
}

export interface ConfigurePayload extends WebsocketPayloadBase {
  event: "client.configure";
  data: Configuration;
}

export interface LighthouseFoundPayload extends WebsocketPayloadBase {
  event: "lighthouse.found";
  data: LighthouseStation;
}


export interface LighthouseUpdatePayload extends WebsocketPayloadBase {
  event: "lighthouse.update";
  data: {
    id: string,
    field_name: string,
    value: any
  }
}

export interface ClientPlatformPayload extends WebsocketPayloadBase {
  event: "client.platform";
  data: {
    system: "windows" | "unix" | "darwin",
    flags: string,
    version: string
  };
}

export interface LighthousePowerStateUpdatePayload extends WebsocketPayloadBase  {
  event: "lighthouse.update.power";
  data: {
    id: string;
    power: "awake" | "sleep" | "standingby"
  }
}


export interface SteamVRStatusUpdatePayload extends WebsocketPayloadBase {
  event: 'steamvr.status',
  data: {
    status: boolean
  }
}

export interface GroupAddedPayload extends WebsocketPayloadBase {
  event: 'groups.created',
  data: LighthouseGroup
}


export interface GroupRenamePayload extends WebsocketPayloadBase {
  event: 'group.rename',
  data: {
    old_name: string,
    new_name: string
  }
}

export interface GroupRenamePayload extends WebsocketPayloadBase {
  event: 'group.rename',
  data: {
    old_name: string,
    new_name: string
  }
}


export interface GroupLighthousesAdded extends WebsocketPayloadBase {
  event: 'groups.lighthouses.added',
  data: {
    id: string,
    group: string,
  }
}

export interface GroupsUpdatedFlagsPayload extends WebsocketPayloadBase {
  event: 'group.update.flags',
  data: {
    name: string;
    flags: number;
  }
}


export type WebsocketPayload =
  | ConfigurePayload
  | SteamVRStatusUpdatePayload
  | ClientPlatformPayload
  | GroupAddedPayload
  | GroupRenamePayload
  | GroupLighthousesAdded
  | GroupsUpdatedFlagsPayload
  | LighthouseFoundPayload
  | LighthouseUpdatePayload;

export type ClientPayload = LighthousePowerStateUpdatePayload;