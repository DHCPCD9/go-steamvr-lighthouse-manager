export namespace main {
	
	export class BaseStationConfiguration {
	    mac_address: string;
	    // Go type: time
	    last_seen: any;
	    channel: number;
	    nickname: string;
	    id: string;
	    managed_flags: number;
	
	    static createFrom(source: any = {}) {
	        return new BaseStationConfiguration(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.mac_address = source["mac_address"];
	        this.last_seen = this.convertValues(source["last_seen"], null);
	        this.channel = source["channel"];
	        this.nickname = source["nickname"];
	        this.id = source["id"];
	        this.managed_flags = source["managed_flags"];
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}
	export class Group {
	    Name: string;
	    managed_flags: number;
	    base_stations: string[];
	
	    static createFrom(source: any = {}) {
	        return new Group(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.Name = source["Name"];
	        this.managed_flags = source["managed_flags"];
	        this.base_stations = source["base_stations"];
	    }
	}
	export class Configuration {
	    is_steamvr_managed: boolean;
	    is_steamvr_installed: boolean;
	    allow_tray: boolean;
	    tray_notified: boolean;
	    known_base_stations: Record<string, BaseStationConfiguration>;
	    groups: Record<string, Group>;
	
	    static createFrom(source: any = {}) {
	        return new Configuration(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.is_steamvr_managed = source["is_steamvr_managed"];
	        this.is_steamvr_installed = source["is_steamvr_installed"];
	        this.allow_tray = source["allow_tray"];
	        this.tray_notified = source["tray_notified"];
	        this.known_base_stations = this.convertValues(source["known_base_stations"], BaseStationConfiguration, true);
	        this.groups = this.convertValues(source["groups"], Group, true);
	    }
	
		convertValues(a: any, classs: any, asMap: boolean = false): any {
		    if (!a) {
		        return a;
		    }
		    if (a.slice && a.map) {
		        return (a as any[]).map(elem => this.convertValues(elem, classs));
		    } else if ("object" === typeof a) {
		        if (asMap) {
		            for (const key of Object.keys(a)) {
		                a[key] = new classs(a[key]);
		            }
		            return a;
		        }
		        return new classs(a);
		    }
		    return a;
		}
	}

}

