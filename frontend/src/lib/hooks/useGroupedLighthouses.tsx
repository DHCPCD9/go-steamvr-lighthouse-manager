import { useContext, useEffect, useState } from "preact/hooks"
import { WebsocketContext } from "../context/websocket.context"


export const useGroupedLighthouses = (groupName: string) => {

    const { groups, lighthouses } = useContext(WebsocketContext);

    if (!groups[groupName]) return [];
    let group = groups[groupName];

    if (!group) return [];
    return Object.values(lighthouses).filter((c) => c && group.base_stations.includes(c.id))
}