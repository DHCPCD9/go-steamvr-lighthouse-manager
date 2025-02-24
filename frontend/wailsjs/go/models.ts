export namespace main {
	
	export class BaseStation {
	    Name: string;
	    PowerState: number;
	    Channel: number;
	    IsValidLighthouse: boolean;
	    Address: string;
	
	    static createFrom(source: any = {}) {
	        return new BaseStation(source);
	    }
	
	    constructor(source: any = {}) {
	        if ('string' === typeof source) source = JSON.parse(source);
	        this.Name = source["Name"];
	        this.PowerState = source["PowerState"];
	        this.Channel = source["Channel"];
	        this.IsValidLighthouse = source["IsValidLighthouse"];
	        this.Address = source["Address"];
	    }
	}

}

