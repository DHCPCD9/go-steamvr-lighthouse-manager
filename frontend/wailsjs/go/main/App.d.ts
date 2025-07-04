// Cynhyrchwyd y ffeil hon yn awtomatig. PEIDIWCH Â MODIWL
// This file is automatically generated. DO NOT EDIT
import {main} from '../models';

export function AddBaseStationToGroup(arg1:string,arg2:string):Promise<string>;

export function ChangeBaseStationChannel(arg1:string,arg2:number):Promise<string>;

export function ChangeBaseStationPowerStatus(arg1:string,arg2:string):Promise<string>;

export function CreateGroup(arg1:string,arg2:Array<string>):Promise<string>;

export function ForceUpdate():Promise<void>;

export function ForgetBaseStation(arg1:string):Promise<void>;

export function GetConfiguration():Promise<main.Configuration>;

export function GetFoundBaseStations():Promise<Record<string, main.JsonBaseStation>>;

export function GetVersion():Promise<string>;

export function IdentitifyBaseStation(arg1:string):Promise<string>;

export function InitBluetooth():Promise<boolean>;

export function IsSteamVRConnected():Promise<boolean>;

export function IsSteamVRConnectivityAvailable():Promise<boolean>;

export function IsUpdatingSupported():Promise<boolean>;

export function Notify(arg1:string,arg2:string):Promise<void>;

export function RemoveGroup(arg1:string):Promise<void>;

export function RenameGroup(arg1:string,arg2:string):Promise<string>;

export function Shutdown():Promise<void>;

export function SleepAllBaseStations():Promise<void>;

export function StartScanFor10Seconds():Promise<void>;

export function ToggleSteamVRManagement():Promise<main.Configuration>;

export function ToggleTray():Promise<main.Configuration>;

export function UpdateBaseStationParam(arg1:string,arg2:string,arg3:any):Promise<void>;

export function UpdateConfigValue(arg1:string,arg2:any):Promise<void>;

export function UpdateGroupManagedFlags(arg1:string,arg2:number):Promise<string>;

export function WakeUpAllBaseStations():Promise<void>;
