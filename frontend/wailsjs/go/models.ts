export namespace main {
	
	export class Configuration {
	    is_steamvr_managed: boolean;
	    is_steamvr_installed: boolean;
	    allow_tray: boolean;
	
	    static createFrom(source: any = {}) {
	        return new Configuration(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.is_steamvr_managed = source["is_steamvr_managed"];
	        this.is_steamvr_installed = source["is_steamvr_installed"];
	        this.allow_tray = source["allow_tray"];
	    }
	}

}

