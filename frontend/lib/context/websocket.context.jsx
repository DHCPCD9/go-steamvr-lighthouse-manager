import { createContext } from 'preact'
import { useEffect, useRef, useState } from 'preact/hooks'

export const WebsocketContext = createContext({
    lighthouses: [],
    steamvr: {
        active: false
    },
    groups: {

    },
    configuration: {

    },
    websocket: {
        ready: false
    }
});

export const WebsocketProvider = ({ children }) => {


    const [state, setState] = useState({
        lighthouses: {},
        groups: {},
        steamvr: {
            active: false
        },
        configuration: {

        },
        websocket: {
            ready: false
        }
    });

    const handleMessage = async (data) => {
        if (data.packet_type == "lighthouse.found") {
            if (!data.data) return;
            setState((state) => { return { ...state, lighthouses: { ...state.lighthouses, [data.data.id]: data.data}} })
        }

        if (data.packet_type.startsWith("lighthouse.update.")) {
            let field = data.packet_type.replace("lighthouse.update.", "");
            
            setState((state) => {
                return {
                    ...state,
                    lighthouses: {
                        ...state.lighthouses,
                        [data.data.id]: {...state.lighthouses[data.data.id], [field]: data.data[field]}
                    }
                }
            });
        }

        if (data.packet_type == "groups.created") {
            if (!data.data) return;
            
            setState((state) => { return { ...state, groups: { ...state.groups, [data.data.name]: data.data}}})
        }

        if (data.packet_type == "group.rename") {
            // setState((state) => {
            //     state.groups[data.data.new_name] = state.groups[data.data.old_name];
            //     return state;
            // });
            setState((state) => { return { ...state, groups: { ...state.groups, [data.data.old_name]: undefined, [data.data.new_name]: {...state.groups[data.data.old_name], name: data.data.new_name}}}})

        }

        
        if (data.packet_type == "groups.lighthouses.added") {
            let group = state.groups[data.data.old_name]

            setState((state) => { return { ...state, groups: { ...state.groups, [data.data.group]: {...group, base_stations: [...state.groups[data.data.group].base_stations, data.data.id]}}}})
        }

        if (data.packet_type == "group.update.flags") {
            let group = state.groups[data.data.name]

            setState((state) => { return { ...state, groups: { ...state.groups, [data.data.name]: {...state.groups[data.data.name], managed_flags: data.data.flags}}}})
        }

        if (data.packet_type == "config.configure") {
            if (!data.data) return;

            setState((state) => { return { ...state, configuration: data.data }})
        }

        if (data.packet_type == "steamvr.status") {
            setState((state) => { return { ...state, steamvr: { active: data.data.status }}})
        }

    
    }

    const ws = useRef(null);

    useEffect(() => {
        let socket;
        (async () => {
            socket = new WebSocket("ws://localhost:15065/api/lighthouse/websocket")

            socket.onopen = () => setState((state) => {
                state.websocket.ready = true

                return state;
            });
            socket.onclose = () => setState((state) => {
                state.websocket.ready = false

                return state;
            });
            socket.onmessage = (e) => {
                handleMessage(JSON.parse(e.data));
            };

            ws.current = socket
        })()
        return () => {
            socket.close()
        }
    }, []);


    return (<WebsocketContext.Provider value={state}>
        {children}
    </WebsocketContext.Provider>);
}
