import { useContext, useEffect, useState } from "preact/hooks"
import { WebsocketContext } from "../context/websocket.context"


export const useLighthouseGroups = () => {

    const { groups } = useContext(WebsocketContext);

    return Object.values(groups).filter(c => c)
}