export namespace main {
	
	export class Configuration {
	    is_steamvr_managed: boolean;
	
	    static createFrom(source: any = {}) {
	        return new Configuration(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.is_steamvr_managed = source["is_steamvr_managed"];
	    }
	}

}

