import { createContext } from 'preact'
import { useEffect, useRef, useState } from 'preact/hooks'
import type { ClientPayload, ClientPlatformPayload, Configuration, LighthouseStation, WebsocketPayload } from "@src/lib/types/index"
import type { ReactNode } from 'react'



export interface WebsocketContextType {
    lighthouses: {
        [name: string]: LighthouseStation
    },
    steamvr: {
        active: boolean
    },
    groups: {
        [name: string]: any
    },
    configuration: Configuration,
    websocket: {
        ready: boolean,
    },
    platform: ClientPlatformPayload['data'],
    send: (payload: ClientPayload) => void
}

const DEFAULT_CONTEXT_VALUE: WebsocketContextType = {
    lighthouses: {},
    steamvr: {
        active: false
    },
    groups: {

    },
    configuration: {
        allow_tray: false,
        groups: {},
        is_steamvr_installed: false,
        is_steamvr_managed: false,
        known_base_stations: {},
        tray_notified: false
    },
    websocket: {
        ready: false
    },
    platform: {
        system: "windows",
        flags: "",
        version: ""
    },
    send: (payload) => {
        console.log("Not ready!")
    }
};

export const WebsocketContext = createContext<WebsocketContextType>(DEFAULT_CONTEXT_VALUE);

export const WebsocketProvider = ({ children }: { children: ReactNode }) => {


    const [state, setState] = useState<WebsocketContextType>(DEFAULT_CONTEXT_VALUE);

    const handleMessage = async (data: WebsocketPayload) => {
        console.log(data)
        if (data.event == "lighthouse.found") {
            if (!data.data) return;

            setState((state) => { return { ...state, lighthouses: { ...state.lighthouses, [data.data.id]: data.data } } })
        }

        if (data.event == "lighthouse.update") {


            setState((state) => {
                return {
                    ...state,
                    lighthouses: {
                        ...state.lighthouses,
                        [data.data.id]: { ...state.lighthouses[data.data.id], [data.data.field_name]: data.data.value }
                    }
                }
            });
        }

        if (data.event == "groups.created") {
            if (!data.data) return;

            setState((state) => { return { ...state, groups: { ...state.groups, [data.data.name]: data.data}}})
        }

        if (data.event == "group.rename") {
            // setState((state) => {
            //     state.groups[data.data.new_name] = state.groups[data.data.old_name];
            //     return state;
            // });
            setState((state) => { return { ...state, groups: { ...state.groups, [data.data.old_name]: undefined, [data.data.new_name]: {...state.groups[data.data.old_name], name: data.data.new_name}}}})

        }


        if (data.event == "groups.lighthouses.added") {
            let group = state.groups[data.data.group]

            setState((state) => { return { ...state, groups: { ...state.groups, [data.data.group]: {...group, base_stations: [...state.groups[data.data.group].base_stations, data.data.id]}}}})
        }

        if (data.event == "group.update.flags") {
            setState((state) => { return { ...state, groups: { ...state.groups, [data.data.name]: {...state.groups[data.data.name], managed_flags: data.data.flags}}}})
        }

        if (data.event == "client.configure") {
            if (!data.data) return;

            setState((state) => { return { ...state, configuration: data.data } })
        }

        if (data.event == "client.platform") {
            if (!data.data) return;

            setState((state) => { return { ...state, platform: data.data } })
        }

        if (data.event == "steamvr.status") {
            setState((state) => { return { ...state, steamvr: { active: data.data.status }}})
        }


    }

    const ws = useRef<WebSocket>(null);

    useEffect(() => {
        let socket: WebSocket;

        const createSocket = (): WebSocket => {
            socket = new WebSocket("ws://localhost:15065/api/lighthouse/websocket")

            socket.onopen = () => {
                setState((state) => {
                    return {
                        ...state, websocket: { ready: true }, send: (payload) => {
                            socket.send(JSON.stringify(payload))
                        }
                    };
                });
            }
            socket.onclose = () => {
                setState(DEFAULT_CONTEXT_VALUE);

                setTimeout(() => {
                    ws.current = createSocket();
                    console.log("Reconnecting...");
                }, 1000)
            }
            socket.onmessage = (e) => {
                handleMessage(JSON.parse(e.data));
            };

            return socket;
        }
        (async () => {
            ws.current = createSocket();
        })()
        return () => {
            socket.close()
        }
    }, []);


    return (<WebsocketContext.Provider value={state}>
        {children}
    </WebsocketContext.Provider>);
}
