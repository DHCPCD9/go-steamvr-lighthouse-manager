import { useContext, useEffect, useState } from "preact/hooks"
import { WebsocketContext } from "../context/websocket.context"


export const useLighthouseGroup = (groupName) => {

    const { groups } = useContext(WebsocketContext);

    return groups[groupName]
}